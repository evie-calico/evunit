SECTION "Opcode Test", ROM0
OpcodeTest:
	REPT 257
		inc bc
	ENDR
	stop
