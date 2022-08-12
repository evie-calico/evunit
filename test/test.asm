SECTION "Opcode Test", ROM0
OpcodeTest:
	inc b
	REPT 257
		add a, b
		jr nc, :+
		stop
		:
	ENDR
	stop
