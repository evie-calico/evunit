SECTION "Tests", ROM0
OpcodeTest:
	add a, b
	ret

DebugTest:
	ld d, d
.exit
	ld d, d
	ret

CrashTest:
	rst crash

Timeout:
	jr Timeout

SECTION "Crash", ROM0[$0038]
crash:
	jr crash
