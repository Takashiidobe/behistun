.globl _start
_start:
	move.l %a0, %usp
	move.l %usp, %a1
	ext.w %d0
	ext.l %d1
	nbcd %d0
	nbcd (%a0)
	swap %d2
	pea (%a0)
	pea 8(%a1)
	move.b %d0, %d1
	move.w (%a0), %d2
	move.l #0x12345678, %d3
	movea.w (%a0), %a1
	movea.l #0x12345678, %a2
	movep.w %d0, 0x8(%a0)
	movep.l 0x10(%a1), %d2
	move.w %sr, %d0
	move.w %d0, %ccr
	move.w %d0, %sr
	orib #16,%ccr
 	oriw #1792,%sr
 	orib #-128,%d0
 	oriw #-3856,%a0@
 	oril #-1430532899,%d1
	eori #0x1f, %ccr
	eori #0x2700, %sr
	eori.b #0xaa, %d0
	eori.w #0x5555, (%a0)
	eori.l #0x12345678, %d1
	cmpi.b #10, %d0
	cmpi.w #0x1000, (%a1)
	cmpi.l #0xdeadbeef, %d2
	andi.b #0x0f, %d0
	andi.w #0x1234, (%a0)
	andi.l #0x12345678, %d1
	subi.b #5, %d2
	subi.w #100, (%a1)
	addi.l #0xaabbccdd, %d3
	sub.w %d1, %d2
	sub.l (%a0), %d3
	sub.b %d5, (%a0)
	subx.w %d1, %d2
	subx.l %d0, %d3
	suba.w (%a0), %a2
	suba.l (%a1), %a2
	addq.b #1, %d0
	addq.w #4, (%a0)
	addq.l #8, 8(%a1)
	subq.b #3, %d1
	subq.w #7, (%a2)
	subq.l #2, %a3
	bchg #3, (%a0)
	bclr #4, (%a0)
	btst #7, %d0
	bset #0, 8(%a1)
	bchg %d0, %d1
	bclr %d1, (%a0)
	bset %d2, 8(%a0)
	btst %d3, %d4
	trap #10
	link.w %a6, #-12
	unlk %a0
	negx.w %d0 
	clr.w %d0 
	neg.w %d0 
	not.w %d1
	lsl.b %d0, %d0
	lsr.b %d0, %d0
	asr.b %d0, %d0
	ror.b %d0, %d0
	adda.w (%a0), %a2
	adda.l (%a1), %a2
  add.w %d1, %d2
  add.l (%a0), %d3
  add.b %d5, (%a0)
  addx.w %d1, %d2
  addx.l %d0, %d3
  addx.w %d5, %d0
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
  .short 0xd03c        /* add.b #0xaa, %d0 */
  .short 0x00aa
  .short 0xd0bc        /* add.l #0x12345678, %d0 */
  .long 0x12345678
  tst.b %d0
  tst.w (%a0)
  tst.l 8(%pc)
  bchg %d0, %d1
  bclr %d1, (%a0)
  bset %d2, 8(%a0)
  bchg #3, (%a0)
  bclr #4, (%a0)
