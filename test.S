.globl _start
_start:
  mulu.w %d0, %d1
  mulu.w (%a0), %d2
  mulu.w #100, %d3
  muls.w %d4, %d5
  muls.w (%a1), %d6
  muls.w 0x1000.w, %d7
  abcd %d0, %d1
  abcd -(%a0), -(%a1)
  exg %d0, %d1
  exg %a0, %a1
  exg %d2, %a2
  and.b %d0, %d1
  and.w (%a0), %d2
  and.l #0x12345678, %d3
  and.b %d4, (%a2)
  and.w %d5, 0x2000.w
  and.l %d6, (%a3)+
  cmp.b %d0, %d1
  cmp.w (%a0), %d2
  cmp.l 0x1000.w, %d3
  cmp.w #100, %d4
  cmpa.w %d0, %a1
  cmpa.l (%a2), %a3
  cmpa.w #0x1234, %a4
  cmpm.b (%a0)+, (%a1)+
  cmpm.w (%a2)+, (%a3)+
  cmpm.l (%a4)+, (%a5)+
  eor.b %d0, %d1
  eor.w %d2, (%a0)
  eor.l %d3, 0x2000.w
  divu.w %d0, %d1
  divu.w (%a0), %d2
  divu.w 0x1000.w, %d3
  divs.w %d4, %d5
  divs.w (%a1), %d6
  divs.w #100, %d7
  sbcd %d0, %d1
  sbcd -(%a0), -(%a1)
  or.b %d0, %d1
  or.w (%a0), %d2
  or.l #0x12345678, %d3
  or.b %d4, (%a2)
  or.w %d5, 0x2000.w
  or.l %d6, (%a3)+
  st %d0
  sf %d1
  shi (%a0)
  sls 0x1000.w
  seq %d2
  sne %d3
  dbt %d0, .+10
  dbf %d1, .-20
  dbeq %d2, .+100
  dbne %d3, .-50
  bra .+10
  bra .+200
  bsr .-100
  bsr .+1000
  beq .+20
  bne .-30
  bhi .+40
  bls .-50
  bge .+60
  blt .-70
  addq.b #1, %d0
  addq.w #3, %a1
  addq.l #8, (%a0)
  addq.w #5, 0x1000.w
  subq.b #2, %d1
  subq.w #4, %a2
  subq.l #7, (%a3)+
  moveq #0, %d0
  moveq #127, %d1
  moveq #-1, %d2
  moveq #-128, %d3
  lea (%a0), %a1
  lea 0x10(%a2), %a3
  lea 0x1234.w, %a4
  lea 0x12345678.l, %a5
  chk.w %d0, %d1
  chk.w (%a0), %d2
  chk.w 0x100.w, %d3
  movem.w %d0-%d3, (%a0)
  movem.l %d0-%d7/%a0-%a6, -(%a7)
  movem.w (%a0), %d0-%d3
  movem.l (%a7)+, %d0-%d7/%a0-%a6
  movem.l 0x1000.w, %d0/%d2/%a1
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
