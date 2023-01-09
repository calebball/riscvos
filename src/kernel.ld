OUTPUT_ARCH( "riscv" )

ENTRY( _start )

MEMORY
{
	ram (wxa) : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS
{
	.text : {
		*(.text.init)
		*(.text .text.*)
	} >ram AT>ram

	.rodata : {
		*(.rodata .rodata.*)
	} >ram AT>ram

	.data : {
		*(.sdata .sdata.*)
		*(.data .data.*)
	} >ram AT>ram

	.bss : {
		*(.sbss .sbss.*) *(.bss .bss.*)
		PROVIDE(_bss_end = .);
	} >ram AT>ram

	PROVIDE(_memory_start = ORIGIN(ram));

	PROVIDE(_stack_start = _bss_end);
	PROVIDE(_stack_end = _stack_start + 0x80000);

	PROVIDE(_heap_start = _stack_end);
	PROVIDE(_heap_end = ORIGIN(ram) + LENGTH(ram));
}
