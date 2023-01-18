OUTPUT_ARCH( "riscv" )

ENTRY( _start )

MEMORY
{
	ram (wxa) : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS
{
	.text : {
		PROVIDE(_text_start = .);
		*(.text.init)
		*(.text .text.*)
		PROVIDE(_text_end = .);
	} >ram AT>ram

	.rodata : {
		. = ALIGN(4096);
		PROVIDE(_rodata_start = .);
		*(.rodata .rodata.*)
		PROVIDE(_rodata_end = .);
	} >ram AT>ram

	.data : {
		. = ALIGN(4096);
		PROVIDE(_data_start = .);
		*(.sdata .sdata.*)
		*(.data .data.*)
		PROVIDE(_data_end = .);
	} >ram AT>ram

	.bss : {
		. = ALIGN(4096);
		PROVIDE(_bss_start = .);
		*(.sbss .sbss.*) *(.bss .bss.*)
		PROVIDE(_bss_end = .);
	} >ram AT>ram

	PROVIDE(_memory_start = ORIGIN(ram));

	PROVIDE(_stack_start = ALIGN(_bss_end, 4096));
	PROVIDE(_stack_end = _stack_start + 0x80000);

	PROVIDE(_heap_start = _stack_end);
	PROVIDE(_heap_end = ORIGIN(ram) + LENGTH(ram));
}
