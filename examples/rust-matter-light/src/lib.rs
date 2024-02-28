// Copyright (C) 2024 Maik Lorenz
//
// This file is subject to the terms and conditions of the GNU Lesser
// General Public License v2.1. See the file LICENSE in the top level
// directory for more details.
#![no_std]
#[allow(warnings)]
#[allow(unused)]

// internal modules
pub mod gpio;
pub mod light;
pub mod saul_reg;
mod dev_att;
mod network;
mod persist;
use network::RiotSocket;
use network::utils::{get_ipv6_address, initialize_network, test_udp, test_udp_wrapper};

// core library
use core::{ffi::CStr, borrow::Borrow, pin::pin};

// external crates
use static_cell::StaticCell;
use embassy_futures::select::{select, select3};
use embedded_nal_async::UdpStack as _;
use embedded_hal::blocking::delay::DelayMs as _;

// RIOT OS modules
extern crate rust_riotmodules;
use riot_wrappers::{riot_main, println};
use riot_wrappers::shell::{self, CommandList};
use riot_wrappers::saul::registration::register_and_then;
use riot_wrappers::ztimer;

// rs-matter
extern crate rs_matter;
use rs_matter::utils::select::EitherUnwrap as _;
use rs_matter::transport::network::{UdpReceive as _, UdpSend as _};
use rs_matter::core::{CommissioningData, Matter};
use rs_matter::data_model::cluster_basic_information::BasicInfoConfig;
use rs_matter::data_model::cluster_on_off;
use rs_matter::data_model::device_types::DEV_TYPE_ON_OFF_LIGHT;
use rs_matter::data_model::objects::*;
use rs_matter::data_model::root_endpoint;
use rs_matter::data_model::system_model::descriptor;
use rs_matter::error::Error as MatterError;
use rs_matter::mdns::builtin::{
    MDNS_IPV4_BROADCAST_ADDR, MDNS_IPV6_BROADCAST_ADDR, MDNS_SOCKET_BIND_ADDR,
};
use rs_matter::mdns::MdnsService;
use rs_matter::transport::core::{PacketBuffers, MATTER_SOCKET_BIND_ADDR};
use rs_matter::transport::network::UdpBuffers;
use rs_matter::MATTER_PORT;
use rs_matter::secure_channel::spake2p::VerifierData;


// Application-specific constants
// User LED port and pin on used boards
const LED_PORT: u32 = 0;
const LED_PIN: u32 = 5;

// RGB LED port and pin
const RGB_PORT: u32 = 0;
const RGB_RED_PIN: u32 = 7;
const RGB_GREEN_PIN: u32 = 6;
const RGB_BLUE_PIN: u32 = 5;

const NODE: Node<'static> = Node {
    id: 0,
    endpoints: &[
        root_endpoint::endpoint(0),
        Endpoint {
            id: 1,
            device_type: DEV_TYPE_ON_OFF_LIGHT,
            clusters: &[descriptor::CLUSTER, cluster_on_off::CLUSTER],
        },
    ],
};

fn matter_handler<'a>(matter: &'a Matter<'a>) -> impl Metadata + NonBlockingHandler + 'a {
    (
        NODE,
        root_endpoint::handler(0, matter)
            .chain(
                1,
                descriptor::ID,
                descriptor::DescriptorCluster::new(*matter.borrow()),
            )
            .chain(
                1,
                cluster_on_off::ID,
                cluster_on_off::OnOffCluster::new(*matter.borrow()),
            ),
    )
}

riot_main!(main);
fn main() -> ! {
    let mut delay = ztimer::Clock::msec();
    delay.delay_ms(1200);
    println!("Hello Matter on RIOT!");

    let mut board_led = gpio::get_output(LED_PORT, LED_PIN);
    board_led.set_high();

    static EXECUTOR: StaticCell<embassy_executor_riot::Executor> = StaticCell::new();
    let executor: &'static mut _ = EXECUTOR.init(embassy_executor_riot::Executor::new());
    executor.run(|spawner| {
        spawner.spawn(amain(spawner)).unwrap();
    });

    let rgb_led = saul_reg::RgbLed::new(
        "Color Temperature Light",
        (RGB_PORT, RGB_RED_PIN),
        (RGB_PORT, RGB_GREEN_PIN),
        (RGB_PORT, RGB_BLUE_PIN),
    );

    register_and_then(
        &saul_reg::RGB_LED_DRIVER,
        &rgb_led,
        Some(CStr::from_bytes_with_nul(b"Color Temperature Light\0").unwrap()),
        || {
            println!("RGB LED registered as SAUL actuator");
            shell::new().run_forever_providing_buf();
        },
    );
}

#[embassy_executor::task]
async fn amain(_spawner: embassy_executor::Spawner) {
    println!("Running async main...");
    run_matter().await;
    println!("Matter exited");
}

async fn run_matter() {
    println!("Network Stack init...");

    // 1. Initialize network stack
    let (ipv4_addr, ipv6_addr, interface) = initialize_network().expect("Error initializing network");
    
    static UDP_MDNS_SOCKET: StaticCell<riot_sys::sock_udp_t> = StaticCell::new();
    let udp_mdns_stack = riot_wrappers::socket_embedded_nal_async_udp::UdpStack::new(|| UDP_MDNS_SOCKET.try_uninit());
    println!("UDP stack created (mDNS)");
    
    static UDP_MATTER_SOCKET: StaticCell<riot_sys::sock_udp_t> = StaticCell::new();
    let udp_matter_stack = riot_wrappers::socket_embedded_nal_async_udp::UdpStack::new(|| UDP_MATTER_SOCKET.try_uninit());
    println!("UDP stack created (Matter)");
    
    let (mdns_addr, mut mdns_sock) = udp_mdns_stack
        .bind_single(MDNS_SOCKET_BIND_ADDR)
        .await
        .expect("Can't create a socket");
    println!("Bound mDNS address: {:?}", &mdns_addr);
    //test_udp(&mut mdns_sock).await;
    let mut mdns_sock_wrapper = RiotSocket::new(mdns_addr, mdns_sock);
    test_udp_wrapper(&mut mdns_sock_wrapper).await;
    return;
    
    let (matter_addr, mut matter_sock) = udp_matter_stack
        .bind_single(MATTER_SOCKET_BIND_ADDR)
        .await
        .expect("Can't create a socket");
    println!("Bound Matter address: {:?}", &matter_addr);
    let mut matter_sock_wrapper = RiotSocket::new(matter_addr, matter_sock);

    // let mut matter_test_runner = pin!(test_udp(&mut matter_sock));
    let mut mdns_test_runner = pin!(test_udp_wrapper(&mut mdns_sock_wrapper));
    //let mut matter_test_runner = pin!(test_udp_wrapper(&mut matter_sock_wrapper));
    let mut matter_test_runner = pin!(async { println!("test"); loop{} });
    select(&mut mdns_test_runner, &mut matter_test_runner).await;
    return;
    
    // 2. Define Product Info
    let dev_det = BasicInfoConfig {
        vid: 0xFFF1,
        pid: 0x8000,
        hw_ver: 2,
        sw_ver: 1,
        sw_ver_str: "1",
        serial_no: "aabbccddd",
        device_name: "OnOff Light",
        product_name: "Light123",
        vendor_name: "Vendor 123",
    };

    // 3. Get Device attestation data
    let dev_att = dev_att::HardCodedDevAtt::new();

    // 4. TODO: Provide own epoch and rand functions
    let epoch = rs_matter::utils::epoch::dummy_epoch;
    let rand = rs_matter::utils::rand::dummy_rand;
    
    // 5. Create mDNS Service
    let mdns = MdnsService::new(
        0,
        "rs-matter-demo",
        ipv4_addr.octets(),
        Some((ipv6_addr.octets(), interface)),
        &dev_det,
        MATTER_PORT,
    );
    
    println!("mDNS initialized!");
    
    // 6. Create Matter struct
    let matter = Matter::new(
        // vid/pid should match those in the DAC
        &dev_det,
        &dev_att,
        &mdns,
        epoch,
        rand,
        MATTER_PORT,
    );
    
    println!("matter initialized!");

    let handler = HandlerCompat(matter_handler(&matter));
    
    // Create mDNS Service Runner
    let mut udp_buffers = UdpBuffers::new();
    // TODO: in function `embassy_time_driver::now': undefined reference to `_embassy_time_now'
    //let mut mdns_runner = pin!(mdns.run(&mdns_sock_wrapper, &mdns_sock_wrapper, &mut udp_buffers));
    let mut mdns_runner = pin!(async {println!("asd")});
    
    let mut udp_buffers = UdpBuffers::new();
    let mut packet_buffers = PacketBuffers::new();
    let comm_data = CommissioningData {
            // TODO: Hard-coded for now
            verifier: VerifierData::new_with_pw(123456, *matter.borrow()),
            discriminator: 250,
    };
    let mut matter_runner = pin!(matter.run(
        &matter_sock_wrapper, 
        &matter_sock_wrapper, 
        &mut udp_buffers, 
        &mut packet_buffers, 
        comm_data, 
        &handler));
    let mut matter_runner = pin!(async {println!("asd")});
    return;

    let mut psm = persist::Psm::new(&matter).expect("Error creating PSM");
    let mut psm_runner = pin!(psm.run());
    
    select3(&mut mdns_runner, &mut matter_runner, &mut psm_runner).await;
    return;
}
