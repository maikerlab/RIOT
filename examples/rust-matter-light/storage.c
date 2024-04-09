#include <stdio.h>
#include <vfs.h>
#include "fs/constfs.h"
#include "constfs_contents.h"

static constfs_file_t constfs_files[] = {
    {
        .path = "/dac",
        .size = sizeof(DAC_CERT),
        .data = (const uint8_t *)DAC_CERT,
    }
};

static constfs_t constfs_desc = {
    .nfiles = ARRAY_SIZE(constfs_files),
    .files = constfs_files,
};

static vfs_mount_t const_mount = {
    .fs = &constfs_file_system,
    .mount_point = "/const",
    .private_data = &constfs_desc,
};

void do_vfs_init(void) {
    int res = vfs_mount(&const_mount);
    if (res < 0) {
        puts("Error while mounting constfs");
    }
    else {
        puts("constfs mounted successfully");
    }
}

#include "board.h"
//#define SDCARD_SPI_PARAM_SPI         SPI_DEV(0)
//#define SDCARD_SPI_PARAM_CS          GPIO_UNDEF
//#define SDCARD_SPI_PARAM_CLK         GPIO_PIN(PORT_A, 5)
//#define SDCARD_SPI_PARAM_MOSI        GPIO_PIN(PORT_A, 7)
//#define SDCARD_SPI_PARAM_MISO        GPIO_PIN(PORT_A, 6)
#include "sdcard_spi.h"
#include "sdcard_spi_internal.h"
#include "sdcard_spi_params.h"

extern sdcard_spi_t sdcard_spi_devs[ARRAY_SIZE(sdcard_spi_params)];
sdcard_spi_t *card = &sdcard_spi_devs[0];

int init_sd(void) {
    printf("Initializing SD-card at SPI_%i...", sdcard_spi_params[0].spi_dev);

    if (sdcard_spi_init(card, &sdcard_spi_params[0]) != 0) {
        printf("[FAILED]");
        printf("enable debugging in sdcard_spi.c for more information!");
        return -2;
    }
    printf("[OK]");
    return 0;
}