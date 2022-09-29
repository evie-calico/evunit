use try_from_discrim::TryFrom;

#[derive(TryFrom)]
#[from(u16)]
#[non_exhaustive]
/// A collection of hardware registers' addresses, extracted from [`hardware.inc`](https://github.com/gbdev/hardware.inc).
pub enum HwReg {
	/// MBC SRAM enable.
	Ramg = 0x0000,
	/// MBC ROM bank switch, low 8 bits.
	Romb0 = 0x2000,
	/// MBC ROM bank switch, upper 8 bits.
	Romb1 = 0x3000,
	/// MBC SRAM bank switch.
	Ramb = 0x4000,
	/// MBC RTC latch toggle.
	Rtclatch = 0x6000,

	/// Joypad.
	P1 = 0xFF00,

	/// Serial data.
	Sb = 0xFF01,
	/// Serial control.
	Sc = 0xFF02,

	/// Divided clock counter.
	Div = 0xFF04,
	/// Timer counter.
	Tima = 0xFF05,
	/// Timer modulo.
	Tma = 0xFF06,
	/// Timer control.
	Tac = 0xFF07,

	/// Pending interrupts.
	If = 0xFF0F,

	/// CH1 frequency sweep.
	Nr10 = 0xFF10,
	/// CH1 duty control & sound length.
	Nr11 = 0xFF11,
	/// CH1 volume control.
	Nr12 = 0xFF12,
	/// CH1 wavelength, low 8 bits.
	Nr13 = 0xFF13,
	/// CH1 wavelength, upper 3 bits & control.
	Nr14 = 0xFF14,
	/// CH2 duty control & sound length.
	Nr21 = 0xFF16,
	/// CH2 volume control.
	Nr22 = 0xFF17,
	/// CH2 wavelength, low 8 bits.
	Nr23 = 0xFF18,
	/// CH2 wavelength, upper 3 bits & control.
	Nr24 = 0xFF19,
	/// CH3 enable.
	Nr30 = 0xFF1A,
	/// CH3 sound length.
	Nr31 = 0xFF1B,
	/// CH3 volume control.
	Nr32 = 0xFF1C,
	/// CH3 wavelength, low 8 bits.
	Nr33 = 0xFF1D,
	/// CH3 wavelength, upper 3 bits.
	Nr34 = 0xFF1E,
	/// CH4 sound length.
	Nr41 = 0xFF20,
	/// CH4 volume control.
	Nr42 = 0xFF21,
	/// CH4 LFSR control.
	Nr43 = 0xFF22,
	/// CH4 control.
	Nr44 = 0xFF23,
	/// Master volume & VIN panning.
	Nr50 = 0xFF24,
	/// Sound panning.
	Nr51 = 0xFF25,
	/// Audio control.
	Nr52 = 0xFF26,

	Wave0 = 0xFF30,
	Wave1 = 0xFF31,
	Wave2 = 0xFF32,
	Wave3 = 0xFF33,
	Wave4 = 0xFF34,
	Wave5 = 0xFF35,
	Wave6 = 0xFF36,
	Wave7 = 0xFF37,
	Wave8 = 0xFF38,
	Wave9 = 0xFF39,
	WaveA = 0xFF3A,
	WaveB = 0xFF3B,
	WaveC = 0xFF3C,
	WaveD = 0xFF3D,
	WaveE = 0xFF3E,
	WaveF = 0xFF3F,

	/// LCD control.
	Lcdc = 0xFF40,
	/// LCD status.
	Stat = 0xFF41,
	/// Viewport vertical offset.
	Scy = 0xFF42,
	/// Viewport horizontal offset.
	Scx = 0xFF43,
	/// Current scanline.
	Ly = 0xFF44,
	/// LY comparison.
	Lyc = 0xFF45,
	/// OAM DMA source & start.
	Dma = 0xFF46,
	/// DMG background palette.
	Bgp = 0xFF47,
	/// DMG OBJ palette 0.
	Obp0 = 0xFF48,
	/// DMG OBJ palette 1.
	Obp1 = 0xFF49,
	/// Window Y coordinate.
	Wy = 0xFF4A,
	/// Window X coordinate.
	Wx = 0xFF4B,

	/// CGB speed switch.
	Key1 = 0xFF4D,

	/// CGB VRAM bank switch.
	Vbk = 0xFF4F,

	/// CGB DMA source, upper 8 bits.
	Hdma1 = 0xFF51,
	/// CGB DMA source, lower 8 bits.
	Hdma2 = 0xFF52,
	/// CGB DMA destination, upper 8 bits.
	Hdma3 = 0xFF53,
	/// CGB DMA destination, lower 8 bits.
	Hdma4 = 0xFF54,
	/// CGB DMA length & mode & start.
	Hdma5 = 0xFF55,

	/// CGB IR.
	Rp = 0xFF56,

	/// CGB BG palette address.
	Bcps = 0xFF68,
	/// CGB BG palette data.
	Bcpd = 0xFF69,
	/// CGB OBJ palette address.
	Ocps = 0xFF6A,
	/// CGB OBJ palette data.
	Ocpd = 0xFF6B,

	/// CGB WRAM bank switch.
	Svbk = 0xFF70,

	/// CH1 & CH2 digital output.
	Pcm12 = 0xFF76,
	/// CH3 & CH4 digital output.
	Pcm34 = 0xFF77,

	/// Enabled interrupts.
	Ie = 0xFFFF,
}

impl From<HwReg> for u16 {
	fn from(reg: HwReg) -> Self {
		reg as u16
	}
}
