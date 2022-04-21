MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  BOOTLOADER                        : ORIGIN = 0x08000000, LENGTH = 24K
  BOOTLOADER_STATE                  : ORIGIN = 0x08006000, LENGTH = 2K
  FLASH                             : ORIGIN = 0x08007000, LENGTH = 114688
  DFU                               : ORIGIN = 0x08023000, LENGTH = 116736
  RAM                         (rwx) : ORIGIN = 0x20000000, LENGTH = 64K
}

__bootloader_state_start = ORIGIN(BOOTLOADER_STATE);
__bootloader_state_end = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE);

__bootloader_dfu_start = ORIGIN(DFU);
__bootloader_dfu_end = ORIGIN(DFU) + LENGTH(DFU);
