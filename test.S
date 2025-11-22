.globl _start
_start:
	bchg #3, (%a0)
	bclr #4, (%a0)
	btst #7, %d0
	bset #0, 8(%a1)
	| trap #10
	| link.w %a6, #-12
	| unlk %a0
	| negx.w %d0 
	| clr.w %d0 
	| neg.w %d0 
	| not.w %d1
	| lsl.b %d0, %d0
	| lsr.b %d0, %d0
	| asr.b %d0, %d0
	| ror.b %d0, %d0
	| adda.w (%a0), %a2
	| adda.l (%a1), %a2
  | add.w %d1, %d2
  | add.l (%a0), %d3
  | add.b %d5, (%a0)
  | addx.w %d1, %d2
  | addx.l %d0, %d3
  | addx.w %d5, %d0
	| illegal
	| reset
	| nop
	| rte
	| rts
	| trapv
	| rtr
  | jsr (%a0)
  | jmp (%a0)
  | jsr 16(%a0)
  | jmp 16(%a0)
  | jsr 8(%a0,%d0)
  | jmp 8(%a0,%d0)
  | jsr _start   
  | jmp _start
  | jsr 16(%pc)
  | jmp 16(%pc)
  | jsr 8(%pc,%d0)
  | jmp 8(%pc,%d0)
  | .short 0xd03c        /* add.b #0xaa, %d0 */
  | .short 0x00aa
  | .short 0xd0bc        /* add.l #0x12345678, %d0 */
  | .long 0x12345678
  | tst.b %d0
  | tst.w (%a0)
  | tst.l 8(%pc)
  | bchg %d0, %d1
  | bclr %d1, (%a0)
  | bset %d2, 8(%a0)
  | bchg #3, (%a0)
  | bclr #4, (%a0)
