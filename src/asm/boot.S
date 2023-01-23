.option norvc

.section .text.init

.global _start
_start:
	la		sp, _stack_end
	call	initialise_kernel

	li		t1, 1 << 11
	csrw	mstatus, t1

	la		t1, kernel_main
	csrw	mepc, t1

	li		t1, 0xffff
	csrw	medeleg, t1
	csrw	mideleg, t1

	li		t1, 0x3fffffffffffff
	csrw	pmpaddr0, t1
	li		t1, 0xf
	csrw	pmpcfg0, t1

	sfence.vma

	mret
