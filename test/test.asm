SECTION "Opcode Test", ROM0
OpcodeTest:
	REPT 257
		add a, 1
		jr nc, :+
		stop
		:
	ENDR
	stop
