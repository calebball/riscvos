.option norvc

.section .rodata

.global STACK_START
STACK_START: .dword _stack_start

.global STACK_END
STACK_END: .dword _stack_end

.global HEAP_START
HEAP_START: .dword _heap_start

.global HEAP_END
HEAP_END: .dword _heap_end
