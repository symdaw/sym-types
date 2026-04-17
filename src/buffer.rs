use std::{mem, sync::atomic::AtomicU64};

pub type SampleRate = u32;
pub type BlockSize = usize;
pub type FrameValue = f32;

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct Buffer {
    pub uid: u64,
    pub data: Vec<Vec<FrameValue>>,
    pointers: Vec<*const FrameValue>,
    pointers_mut: Vec<*mut FrameValue>,
    latency: i32,
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new(2, 0)
    }
}

unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

pub trait AutomationValueAccessor {
    fn try_read_static(&self) -> Option<f32>;
    fn read_sample_accurate(&self, sample_index: usize, block_size: usize) -> f32;
    fn read_static(&mut self) -> f32;
    fn finish(&mut self);
}

impl Buffer {
    pub fn as_raw(&self) -> *const *const FrameValue {
        self.pointers.as_ptr()
    }

    pub fn as_raw_mut(&mut self) -> *mut *mut FrameValue {
        self.pointers_mut.as_mut_ptr()
    }

    pub fn new(num_channels: usize, num_frames: BlockSize) -> Self {
        let uid = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        debug_assert!(num_channels <= 16);

        let data = vec![vec![0.0; num_frames as usize]; num_channels];

        let pointers = data
            .iter()
            .map(|channel| channel.as_ptr())
            .collect::<Vec<_>>();

        let pointers_mut = data
            .iter()
            .map(|channel| channel.as_ptr() as *mut FrameValue)
            .collect::<Vec<_>>();

        Self {
            uid,
            data,
            pointers,
            pointers_mut,
            latency: 0,
        }
    }

    pub fn new_from_raw(channels: &[*mut f32], block_size: BlockSize) -> Self {
        let uid = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        debug_assert!(channels.len() <= 16);

        // let data = vec![vec![0.0; num_frames as usize]; num_channels];

        let data: Vec<Vec<f32>> = unsafe {
            channels
                .iter()
                .map(|c| Vec::from_raw_parts(*c, block_size, block_size))
                .collect()
        };

        let pointers = data
            .iter()
            .map(|channel| channel.as_ptr())
            .collect::<Vec<_>>();

        let pointers_mut = data
            .iter()
            .map(|channel| channel.as_ptr() as *mut FrameValue)
            .collect::<Vec<_>>();

        Self {
            uid,
            data,
            pointers,
            pointers_mut,
            latency: 0,
        }
    }

    pub fn new_with_settings_of(other: &Buffer) -> Self {
        let mut buffer = Self::new(other.channel_count(), other.frame_count());
        buffer.add_latency(other.get_latency());
        buffer
    }

    pub fn add_latency(&mut self, latency: i32) {
        self.latency += latency;
    }

    pub fn get_latency(&self) -> i32 {
        self.latency
    }

    pub fn copy_from(&mut self, other: &Buffer) {
        debug_assert_eq!(self.data.len(), other.data.len());
        debug_assert_eq!(self.data[0].len(), other.data[0].len());
        // debug_assert_eq!(self.latency, other.latency);

        self.latency = other.latency;

        for (i, channel) in self.data.iter_mut().enumerate() {
            for (j, sample) in channel.iter_mut().enumerate() {
                *sample = other.data[i][j];
            }
        }
    }

    pub fn resize(&mut self, new_frame_count: BlockSize) {
        let new_frame_count = new_frame_count as usize;
        for channel in self.data.iter_mut() {
            channel.resize(new_frame_count, 0.0);
        }
    }

    pub fn write(&mut self, channel: usize, frame: usize, value: FrameValue) {
        if channel >= self.channel_count() || frame >= self.frame_count() {
            return;
        }

        self.data[channel][frame] = value;
    }

    pub fn push(&mut self, channel: usize, value: FrameValue) {
        if channel >= self.channel_count() {
            return;
        }

        self.data[channel].push(value);
    }

    pub fn channel_count(&self) -> usize {
        self.data.len()
    }

    pub fn frame_count(&self) -> usize {
        if self.data.is_empty() {
            return 0;
        }

        self.data[0].len()
    }

    pub fn resize_if_needed(&mut self, channel_count: usize, frame_count: usize) {
        self.zero();

        while self.channel_count() > channel_count {
            self.data.pop();
        }

        while self.channel_count() < channel_count {
            self.data.push(Vec::with_capacity(frame_count));
        }

        if self.frame_count() != frame_count {
            // println!("frame resize {}", frame_count);
            for channel in &mut self.data {
                channel.resize(frame_count, 0.);
            }
        }
    }

    /// Gain in dB
    pub fn attenuate(&mut self, gain: &mut impl AutomationValueAccessor) {
        if let Some(gain) = gain.try_read_static() {
            if gain == 0. {
                return;
            }

            let scale = voltage_scale_from_gain(gain);

            for channel in self.data.iter_mut() {
                for sample in channel.iter_mut() {
                    *sample *= scale;
                }
            }

            return;
        }

        // Sample accurate data was provided, so we need to apply it sample by sample

        let block_size = self.frame_count();

        for channel in self.data.iter_mut() {
            for (i, sample) in channel.iter_mut().enumerate() {
                *sample *= voltage_scale_from_gain(gain.read_sample_accurate(i, block_size));
            }
        }
        gain.finish();
    }

    /// 0 = center, -50 = left, 50 = right
    pub fn pan(&mut self, pan: &mut impl AutomationValueAccessor) {
        if self.channel_count() != 2 {
            return;
        }

        // if pan == 0. || self.channel_count() != 2 {
        //     return;
        // }

        if let Some(pan) = pan.try_read_static() {
            if pan == 0. {
                return;
            }

            let pan = (pan / 50.).clamp(-1., 1.);
            let left = (1. - pan) / 2.;
            let right = (1. + pan) / 2.;

            for (i, channel) in self.data.iter_mut().enumerate() {
                for sample in channel.iter_mut() {
                    if i == 0 {
                        *sample *= left;
                    } else {
                        *sample *= right;
                    }
                }
            }

            return;
        }

        // Sample accurate data was provided, so we need to apply it sample by sample

        let block_size = self.frame_count();

        for (channel_i, channel) in self.data.iter_mut().enumerate() {
            for (sample_i, sample) in channel.iter_mut().enumerate() {
                let pan = pan.read_sample_accurate(sample_i, block_size);

                let pan = (pan / 50.).clamp(-1., 1.);
                let left = (1. - pan) / 2.;
                let right = (1. + pan) / 2.;

                if channel_i == 0 {
                    *sample *= left;
                } else {
                    *sample *= right;
                }
            }
        }
        pan.finish();
    }

    pub fn peaks(&self) -> Vec<f32> {
        let mut peaks = Vec::with_capacity(self.channel_count());

        for c in 0..self.channel_count() {
            let (max, min) = self.max_min_amplitude(c);
            let peak = max.abs().max(min.abs());
            peaks.push(peak);
        }

        peaks
    }

    pub fn invert_phase(&mut self) {
        for channel in self.data.iter_mut() {
            for sample in channel.iter_mut() {
                *sample *= -1.;
            }
        }
    }

    /// Limit in dB FS
    pub fn clip(&mut self, limit: f32) {
        let scale = 10.0_f32.powf(limit / 20.);
        for channel in self.data.iter_mut() {
            for sample in channel.iter_mut() {
                *sample = sample.clamp(-scale, scale);
            }
        }
    }

    pub fn stereo_to_mono(&mut self) {
        debug_assert_eq!(self.channel_count(), 2);

        self.data.pop();
    }

    pub fn mono_to_stereo(&mut self) {
        debug_assert_eq!(self.channel_count(), 1);

        self.data.push(self.data.first().unwrap().clone());
    }

    pub fn max_min_amplitude(&self, channel: usize) -> (FrameValue, FrameValue) {
        let channel = &self.data[channel];
        let mut max = FrameValue::MIN;
        let mut min = FrameValue::MAX;

        for sample in channel.iter() {
            max = max.max(*sample);
            min = min.min(*sample);
        }

        (max, min)
    }

    pub fn extend(&mut self, other: &Buffer) {
        debug_assert_eq!(self.channel_count(), other.channel_count());

        if self.frame_count() == 0 {
            self.latency = other.latency;
        }

        debug_assert_eq!(self.latency, other.latency);

        for (i, channel) in self.data.iter_mut().enumerate() {
            channel.extend(other.data[i].iter().cloned());
        }
    }

    /// Flip the left and right channels
    pub fn flip(&mut self) {
        debug_assert_eq!(self.channel_count(), 2, "Can only flip stero signals");

        let temp = mem::take(&mut self.data[0]);
        self.data[0] = mem::take(&mut self.data[1]);
        self.data[1] = temp;
    }

    pub fn is_evil(&self) -> bool {
        for channel in &self.data {
            for frame in channel {
                if !frame.is_finite() {
                    return true;
                }
            }
        }

        false
    }

    // fn _encode_flac(&self, sample_rate: u32) -> Result<flacenc::bitsink::MemSink<u8>, String> {
    //     let channels = self.channel_count();

    //     let config = flacenc::config::Encoder::default()
    //         .into_verified()
    //         .map_err(|_| "Failed flac verification".to_string())?;

    //     let bits_per_sample = 24;

    //     let mut samples = Vec::with_capacity(self.frame_count() * channels);
    //     for i in 0..self.frame_count() {
    //         for j in 0..channels {
    //             samples.push(
    //                 (self.data[j][i].clamp(-1., 1.) * (1 << (bits_per_sample - 1)) as f32) as i32,
    //             );
    //         }
    //     }

    //     let source = flacenc::source::MemSource::from_samples(
    //         samples.as_slice(),
    //         channels,
    //         bits_per_sample,
    //         sample_rate as usize,
    //     );
    //     let flac_stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
    //         .map_err(|_| "Encoding error".to_string())?;

    //     let mut sink = flacenc::bitsink::ByteSink::new();
    //     flac_stream
    //         .write(&mut sink)
    //         .map_err(|_| "Writing error".to_string())?;

    //     Ok(sink)
    // }

    // TODO: Make this realtime safe
    pub fn as_interleaved(&self) -> Vec<FrameValue> {
        let mut interleaved = Vec::with_capacity(self.frame_count() * self.channel_count());

        for i in 0..self.frame_count() {
            for j in 0..self.channel_count() {
                interleaved.push(self.data[j][i]);
            }
        }

        interleaved
    }

    // TODO: Make this realtime safe
    pub fn from_interleaved(&mut self, interleaved: Vec<FrameValue>) {
        let mut interleaved = interleaved.into_iter();

        for i in 0..self.frame_count() {
            for j in 0..self.channel_count() {
                self.data[j][i] = interleaved.next().unwrap();
            }
        }
    }

    pub fn zero(&mut self) {
        self.latency = 0;
        for channel in self.data.iter_mut() {
            for sample in channel.iter_mut() {
                // Multiplying to encourage SIMD. Idk if that's how it works
                *sample *= 0.;
            }
        }
    }

    pub fn force_stereo(mut self) -> Buffer {
        if self.channel_count() == 1 {
            self.data.push(self.data[0].clone());
        } else if self.channel_count() > 2 {
            self.data.splice(2.., []);
        }

        debug_assert_eq!(self.channel_count(), 2);

        self
    }

    pub fn reverse(&mut self) {
        for channel in &mut self.data {
            channel.reverse();
        }
    }
}

impl std::ops::AddAssign<&Buffer> for Buffer {
    fn add_assign(&mut self, rhs: &Buffer) {
        debug_assert_eq!(self.channel_count(), rhs.channel_count());
        debug_assert_eq!(self.frame_count(), rhs.frame_count());
        debug_assert_eq!(self.latency, rhs.latency);

        for (i, channel) in self.data.iter_mut().enumerate() {
            for (j, sample) in channel.iter_mut().enumerate() {
                *sample += rhs.data[i][j];
            }
        }
    }
}

pub fn voltage_scale_from_gain(gain: f32) -> f32 {
    10.0f32.powf(gain / 20.0)
}

pub fn gain_from_voltage_scale(scale: f32) -> f32 {
    scale.log10() * 20.
}
