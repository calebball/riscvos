.option norvc

.section .text.init

.global _start
_start:
	la		sp, _stack
	call	kernel_main
