.globl _start
_start:
	illegal
	reset
	nop
	rte
	rts
	trapv
	rtr
  jsr (%a0)
  jmp (%a0)
  jsr 16(%a0)
  jmp 16(%a0)
  jsr 8(%a0,%d0)
  jmp 8(%a0,%d0)
  jsr _start   
  jmp _start
  jsr 16(%pc)
  jmp 16(%pc)
  jsr 8(%pc,%d0)
  jmp 8(%pc,%d0)
