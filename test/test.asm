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

VariableTest:
	ld a, [wVariable]
	add a, a
	ld [wVariable], a
	ret

StringTest:
	ld hl, wString
	ld de, .string
.loop
	ld a, [de]
	cp a, [hl]
	ret nz
	ld a, [hl]
	and a, a
	ret z
	inc hl
	inc de
	jr .loop

.string db "Hello, world!", 0

HighMemoryTest:
	ld [hVariable], a
	ld a, [hVariable]
	ld b, a
	ret

SECTION "Crash", ROM0[$0038]
crash:
	jr crash

SECTION "Memory", WRAM0
wVariable:
	db

wString: ds strlen("Hello, world!") + 1

SECTION "HighMemory", HRAM
hVariable:
	db