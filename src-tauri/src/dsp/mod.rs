//! §7 DSP 链路（真实信号处理，非桩）：
//! 解码 PCM → EQ 10 段双二阶 → SurroundChain → 30ms 淡变 → −1dBFS 软限幅 → 输出。
//! 另提供人声减弱（karaoke 矩阵）与 rustfft 2048 点频谱（雪天音频响应）。
use std::f32::consts::PI;
use rustfft::{num_complex::Complex, FftPlanner};
use crate::models::{EqConfig, SurroundConfig};

const SR: f32 = 48_000.0;
const LIM: f32 = 0.891_250_9; // −1 dBFS

#[derive(Clone, Copy, Default)]
pub struct Biquad {
    b0: f32, b1: f32, b2: f32, a1: f32, a2: f32,
    x1: f32, x2: f32, y1: f32, y2: f32,
}

impl Biquad {
    /// RBJ Audio EQ Cookbook：peaking EQ / low-pass / high-pass。
    pub fn new(kind: &str, freq: f32, gain_db: f32, q: f32) -> Self {
        let w0 = 2.0 * PI * (freq.clamp(10.0, SR * 0.49) / SR);
        let cw = w0.cos();
        let sw = w0.sin();
        let (b0, b1, b2, a0, a1, a2) = match kind {
            "lowpass" => {
                let alpha = sw / (2.0 * q.max(0.3));
                ((1.0 - cw) / 2.0, 1.0 - cw, (1.0 - cw) / 2.0,
                 1.0 + alpha, -2.0 * cw, 1.0 - alpha)
            }
            "highpass" => {
                let alpha = sw / (2.0 * q.max(0.3));
                ((1.0 + cw) / 2.0, -(1.0 + cw), (1.0 + cw) / 2.0,
                 1.0 + alpha, -2.0 * cw, 1.0 - alpha)
            }
            _ => {
                let a = 10f32.powf(gain_db / 40.0);
                let alpha = sw / (2.0 * q.max(0.3));
                (1.0 + alpha * a, -2.0 * cw, 1.0 - alpha * a,
                 1.0 + alpha / a, -2.0 * cw, 1.0 - alpha / a)
            }
        };
        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0, ..Default::default() }
    }
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1; self.x1 = x;
        self.y2 = self.y1; self.y1 = y;
        if !y.is_finite() { self.reset(); 0.0 } else { y }
    }
    pub fn reset(&mut self) { self.x1 = 0.0; self.x2 = 0.0; self.y1 = 0.0; self.y2 = 0.0 }
}

#[derive(Default)]
pub struct Fade {
    target: f32,
    current: f32,
    step: f32,
}

impl Fade {
    /// 30ms 淡变：任何配置切换 / 播放起停都先淡出再淡入，防爆音。
    pub fn to(&mut self, target: f32) {
        self.target = target.clamp(0.0, 1.0);
        self.step = if (self.target - self.current).abs() < f32::EPSILON { 0.0 }
            else { (self.target - self.current) / (SR * 0.03) };
    }
    #[inline]
    pub fn gain(&mut self) -> f32 {
        if self.step != 0.0 {
            self.current += self.step;
            if (self.step > 0.0 && self.current >= self.target) || (self.step < 0.0 && self.current <= self.target) {
                self.current = self.target; self.step = 0.0;
            }
        }
        self.current.max(0.0)
    }
}

/// EQ 双二阶链：10 段 peaking + HPF + LPF，左右声道独立状态。
pub struct EqChain {
    bands: Vec<(Biquad, Biquad)>,
    hpf: (Biquad, Biquad),
    lpf: (Biquad, Biquad),
    pub enabled: bool,
    strength: f32,
}

impl Default for EqChain {
    fn default() -> Self {
        let flat = || (Biquad::default(), Biquad::default());
        Self { bands: (0..10).map(|_| flat()).collect(), hpf: flat(), lpf: flat(), enabled: false, strength: 0.0 }
    }
}

impl EqChain {
    pub fn apply_config(&mut self, cfg: &EqConfig) {
        let s = (cfg.strength / 100.0).clamp(0.0, 1.0);
        self.strength = s;
        let qs = 0.707f32;
        let mut bands = Vec::with_capacity(10);
        for b in cfg.bands.iter().take(10) {
            let g = b.gain * s;
            let q = if b.q > 0.0 { b.q } else { qs };
            bands.push((
                Biquad::new("peak", b.freq as f32, g, q),
                Biquad::new("peak", b.freq as f32, g, q),
            ));
        }
        while bands.len() < 10 { bands.push((Biquad::default(), Biquad::default())) }
        self.bands = bands;
        self.hpf = (Biquad::new("highpass", cfg.hpf.max(20.0), 0.0, 0.707), Biquad::new("highpass", cfg.hpf.max(20.0), 0.0, 0.707));
        self.lpf = (Biquad::new("lowpass", cfg.lpf.min(20_000.0), 0.0, 0.707), Biquad::new("lowpass", cfg.lpf.min(20_000.0), 0.0, 0.707));
        self.enabled = cfg.bands.iter().any(|b| b.gain.abs() > 0.01) || cfg.hpf > 20.0 || cfg.lpf < 20_000.0;
    }
    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        if !self.enabled { return (l, r) }
        let (mut l, mut r) = (l, r);
        for (bl, br) in &mut self.bands { l = bl.process(l); r = br.process(r) }
        l = self.hpf.0.process(l); r = self.hpf.1.process(r);
        l = self.lpf.0.process(l); r = self.lpf.1.process(r);
        (l, r)
    }
}

/// 3D 环绕：交叉馈送 + 对侧耳 ITD≈0.25ms + 高频滚降；
/// 质数抽头早期反射 7/11/19/29/37ms + 轻量反馈混响；150Hz 以下旁路。
pub struct SurroundChain {
    pub enabled: bool,
    strength: f32,
    width: f32,
    distance: f32,
    taps_ms: Vec<u32>,
    reverb: f32,
    delay: Vec<f32>,
    pos: usize,
    lp: (Biquad, Biquad),
    bass: (f32, f32),
    fb: (f32, f32),
    cross: f32,
}

impl Default for SurroundChain {
    fn default() -> Self {
        Self {
            enabled: false, strength: 0.5, width: 1.2, distance: 1.0,
            taps_ms: vec![7, 11, 19, 29, 37], reverb: 0.3,
            delay: vec![0.0; 4096], pos: 0,
            lp: (Biquad::new("lowpass", 6500.0, 0.0, 0.707), Biquad::new("lowpass", 6500.0, 0.0, 0.707)),
            bass: (0.0, 0.0), fb: (0.0, 0.0), cross: 0.0,
        }
    }
}

impl SurroundChain {
    pub fn apply_config(&mut self, cfg: &SurroundConfig) {
        self.enabled = cfg.enabled;
        self.strength = (cfg.strength / 100.0).clamp(0.0, 1.0);
        self.width = cfg.width.clamp(0.5, 2.0);
        self.distance = cfg.distance.clamp(0.5, 3.0);
        self.reverb = cfg.reverb.clamp(0.0, 1.0);
        self.taps_ms = if cfg.taps.is_empty() { vec![7, 11, 19, 29, 37] } else { cfg.taps.clone() };
        self.cross = 0.28 * self.strength * self.width;
        if self.delay.len() < 4096 { self.delay.resize(4096, 0.0) }
    }
    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        if !self.enabled || self.strength <= 0.0 { return (l, r) }
        // 一阶低通分离低频：150Hz 以下不空间化，原样保留。
        let a = (-2.0 * PI * 150.0 / SR).exp();
        self.bass.0 = a * self.bass.0 + (1.0 - a) * l;
        self.bass.1 = a * self.bass.1 + (1.0 - a) * r;
        let (hl, hr) = (l - self.bass.0, r - self.bass.1);

        let n = self.delay.len();
        self.pos = (self.pos + 1) % n;
        let itd = (SR * 0.00025) as usize; // 0.25ms ≈ 12 samples @48k
        let itd = itd.max(1).min(n - 1);
        let d_l = self.delay[(self.pos + n - itd) % n];
        let d_r = self.delay[self.pos];
        let early = self.taps_ms.iter().enumerate().map(|(i, ms)| {
            let d = ((SR * (*ms as f32) / 1000.0) as usize).min(n - 1);
            self.delay[(self.pos + n - d) % n] * 0.12 / (1 + i) as f32
        }).sum::<f32>();
        self.delay[self.pos] = (hl + hr) * 0.5 + self.fb.0 * self.reverb * 0.35;
        self.fb.0 = self.delay[self.pos];

        let wet = self.strength / self.distance;
        let mut wl = hl + self.cross * (hr + d_l * 0.5) + early * wet;
        let mut wr = hr + self.cross * (hl + d_r * 0.5) + early * wet;
        wl = self.lp.0.process(wl); wr = self.lp.1.process(wr);
        (self.bass.0 + wl, self.bass.1 + wr)
    }
}

#[inline]
pub fn soft_limit(x: f32) -> f32 {
    // 软限幅：接近 −1dBFS 时平滑压缩，绝不硬削波。
    let a = x.abs();
    if a <= LIM { x } else { x.signum() * (LIM + (1.0 - LIM) * (1.0 - (-(a - LIM) / (1.0 - LIM)).exp())) }
}

/// 人声减弱：标准 karaoke 矩阵 L−k·R / R−k·L，100% 抵消中置并回补低频。
pub fn vocal_reduce(pcm: &mut [f32], strength: f32, sample_rate: u32) {
    let k = (strength / 100.0).clamp(0.0, 1.0);
    if k <= 0.0 { return }
    let a = (-2.0 * PI * 180.0 / sample_rate as f32).exp();
    let mut low_l = 0.0f32;
    let mut low_r = 0.0f32;
    for ch in pcm.chunks_exact_mut(2) {
        let (l, r) = (ch[0], ch[1]);
        low_l = a * low_l + (1.0 - a) * l;
        low_r = a * low_r + (1.0 - a) * r;
        let mid = (l + r) * 0.5;
        // 单声道中频衰减 + 矩阵抵消，低频回补避免伴奏塌掉
        let out_l = l - k * r + k * (low_l - mid) * 0.35;
        let out_r = r - k * l + k * (low_r - mid) * 0.35;
        ch[0] = out_l; ch[1] = out_r;
    }
}

/// 2048 点 rustfft + Hann 窗 + 0.78 平滑，返回 {bass, mid, high}（§8 雪天音频响应）。
pub struct Spectrum {
    planner: FftPlanner<f32>,
    smooth: [f32; 3],
}

impl Default for Spectrum {
    fn default() -> Self { Self { planner: FftPlanner::new(), smooth: [0.0; 3] } }
}

impl Spectrum {
    pub fn analyze(&mut self, mono: &[f32], sample_rate: u32) -> [f32; 3] {
        let n = 2048usize;
        let fft = self.planner.plan_fft_forward(n);
        let mut buf: Vec<Complex<f32>> = (0..n).map(|i| {
            let w = 0.5 - 0.5 * (2.0 * PI * i as f32 / (n - 1) as f32).cos();
            Complex::new(mono.get(i).copied().unwrap_or(0.0) * w, 0.0)
        }).collect();
        fft.process(&mut buf);
        let bin = sample_rate as f32 / n as f32;
        let mut bands = [0f32; 3];
        for (i, c) in buf.iter().enumerate().take(n / 2) {
            let f = i as f32 * bin;
            let e = c.norm_sqr();
            if f < 250.0 { bands[0] += e } else if f < 4000.0 { bands[1] += e } else { bands[2] += e }
        }
        let peak = bands.iter().copied().fold(1e-9, f32::max).sqrt();
        for i in 0..3 {
            let v = (bands[i].sqrt() / peak).clamp(0.0, 1.0);
            self.smooth[i] = self.smooth[i] * 0.78 + v * 0.22;
        }
        self.smooth
    }
}
