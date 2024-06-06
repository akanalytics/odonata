use std::any::type_name;
use std::fmt::{Debug, Display};
use std::io::{Read, Write};
use std::ops::{AddAssign, Mul, Neg};
use std::path::Path;

use num_traits::MulAdd;
use odonata_base::infra::math::Quantize;
use odonata_base::infra::utils::{self};
use odonata_base::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use simba::scalar::RealField;

use super::vector::Vector;

pub trait Network {
    type Accumulators: Clone;
    type Input;
    type Output;

    fn new_accumulators(&self) -> Self::Accumulators;
    fn forward1_input(&self, wb: &mut Self::Accumulators, bd1: &Board, bd2: &Board);
    fn forward1(&self, acc: &mut Self::Accumulators, b: &Board);
    fn forward2(&self, pov: Color, state: &Self::Accumulators) -> Self::Output;

    fn predict(&self, bd: &Board) -> Self::Output {
        let mut accs = self.new_accumulators();
        self.forward1(&mut accs, bd);
        self.forward2(bd.turn(), &accs)
    }
}

#[derive(PartialEq, Default, Clone, Serialize, Deserialize)]
#[repr(C, align(64))]
pub struct Network768xH2<T: Default + Copy> {
    pub n_features:     usize,
    pub n_hidden_layer: usize,
    pub wt:             Vec<Vector<T>>, // (acc = hl_size) * input_size
    pub bi:             Vector<T>,
    pub h1_wt:          [Vector<T>; 2], // hl_size
    pub h1_bi:          Vector<T>,      // size 1
    #[serde(default)]
    pub description:    String,
}

impl<T> Display for Network768xH2<T>
where
    T: Copy + Default,
    Vector<T>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(
            f,
            "{} fts:{} hidden:{}",
            utils::type_suffix(type_name::<Self>()),
            self.n_features,
            self.n_hidden_layer
        )?;
        writeln!(f, "description: {}", self.description)?;
        for i in 0..10 {
            writeln!(f, "wt[{i}]  : {:?}", self.wt[i])?;
        }
        writeln!(f, "bi     : {:?}", self.bi)?;
        writeln!(f, "h1[0]  : {:?}", self.h1_wt[0])?;
        writeln!(f, "h1[1]  : {:?}", self.h1_wt[1])?;
        writeln!(f, "h1_bi  : {:?}", self.h1_bi)?;
        Ok(())
    }
}

#[inline(always)]
pub fn crelu<T: RealField>(value: T) -> T {
    T::clamp(value, T::zero(), T::one())
}

impl Network768xH2<i16> {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Box<Self>> {
        let path = path.as_ref().to_string_lossy().into_owned();
        // if path.as_ref().to_str() = Some("") {}
        let mut buf = Vec::new();
        if path.is_empty() {
            debug!(target: "config", "loading nnue from default location");
            let buf = include_bytes!("../../resources/r61-net.i16.bin");
            buf.as_slice();
            NetworkLoader::read_postcard_format(buf)
        } else if path.ends_with("i16.yaml") {
            let net = serde_yaml::from_reader(file_open(&path)?).context(path)?;
            Ok(Box::new(net))
        } else {
            debug!(target: "config", "loading binary nnue from {}", path);
            let _bytes = utils::file_open(&path).unwrap().read_to_end(&mut buf).unwrap();
            NetworkLoader::read_postcard_format(buf.as_slice())
        }
    }
}

impl Network768xH2<Float> {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Box<Self>> {
        let path = path.as_ref();
        if path.to_string_lossy().ends_with(".yaml") {
            Ok(serde_yaml::from_reader(file_open(path)?)?)
        } else {
            anyhow::bail!("unknown file formal {path:?}");
        }
    }
}

/// we accumulate for both perspectives: w+b
///
/// the feature index is w: 0...384, b: 384..768
/// but when from black perspective the color of the piece is flipped
/// and the sq is is flipped too (essentially a board-flip)
#[inline]
pub fn feature768(p: Piece, sq: Square, piece_color: Color) -> (usize, usize) {
    let wpers = piece_color.chooser_wb(0, 384) + p.index() * 64 + sq.index(); // c == w, our perspective
    let bpers = piece_color.chooser_wb(384, 0) + p.index() * 64 + (sq.flip_vertical().index()); // c==b, pers==b => us
    (wpers, bpers)
}

pub fn feature768_diff_iter(b1: &Board, b2: &Board) -> impl Iterator<Item = (Piece, Square, Color)> {
    #[inline(always)]
    fn iter_of(b1: &Board, b2: &Board, p: Piece, c: Color) -> impl Iterator<Item = (Piece, Square, Color)> {
        ((b2.pieces(p) & b2.color(c)) - (b1.pieces(p) & b1.color(c)))
            .squares()
            .map(move |sq| (p, sq, c))
    }

    iter_of(b1, b2, Piece::Pawn, Color::White)
        .chain(iter_of(b1, b2, Piece::Pawn, Color::Black))
        .chain(iter_of(b1, b2, Piece::Knight, Color::White))
        .chain(iter_of(b1, b2, Piece::Knight, Color::Black))
        .chain(iter_of(b1, b2, Piece::Bishop, Color::White))
        .chain(iter_of(b1, b2, Piece::Bishop, Color::Black))
        .chain(iter_of(b1, b2, Piece::Rook, Color::White))
        .chain(iter_of(b1, b2, Piece::Rook, Color::Black))
        .chain(iter_of(b1, b2, Piece::Queen, Color::White))
        .chain(iter_of(b1, b2, Piece::Queen, Color::Black))
        .chain(iter_of(b1, b2, Piece::King, Color::White))
        .chain(iter_of(b1, b2, Piece::King, Color::Black))
}

impl<T> Network768xH2<T>
where
    T: Clone + Default + Copy + Mul<Output = T> + MulAdd<Output = T> + AddAssign + Neg<Output = T> + num_traits::One,
    Vector<T>: for<'a> AddAssign<&'a Vector<T>>,
    Self: for<'a> Deserialize<'a>,
{
    fn new_accumulators(&self) -> (Vector<T>, Vector<T>) {
        (self.bi.clone(), self.bi.clone())
    }

    fn forward1(&self, (w, b): &mut (Vector<T>, Vector<T>), bd: &Board) {
        for sq in bd.occupied().squares() {
            let p = bd.piece_unchecked(sq);
            let c = bd.color_of(sq).unwrap();
            let (wf, bf) = feature768(p, sq, c);
            *w += &self.wt[wf];
            *b += &self.wt[bf];
        }
    }

    fn forward1_input(&self, (w, b): &mut (Vector<T>, Vector<T>), bd1: &Board, bd2: &Board) {
        for (p, sq, c) in feature768_diff_iter(bd1, bd2) {
            let (wi, bi) = feature768(p, sq, c);
            w.mul_add_assign(T::one(), &self.wt[wi]);
            b.mul_add_assign(T::one(), &self.wt[bi]);
        }
        for (p, sq, c) in feature768_diff_iter(bd2, bd1) {
            let (wi, bi) = feature768(p, sq, c);
            w.mul_add_assign(-T::one(), &self.wt[wi]);
            b.mul_add_assign(-T::one(), &self.wt[bi]);
        }
    }
}

impl<T> Network768xH2<T>
where
    T: Copy + Default + DeserializeOwned,
    T: AddAssign + RealField + Mul<Output = T>,
    Vector<T>: Quantize<Output = Vector<i16>>,
{
    pub fn quantize(&self, factors: &[T]) -> Result<Network768xH2<i16>> {
        let n_feat = self.wt.len();
        let n_hidden = self.wt[0].len();
        let mut q = Network768xH2::<i16>::new(n_feat, n_hidden);
        for f in 0..n_feat {
            q.wt[f] = (&self.wt[f] * factors[0]).quantize()?;
        }
        q.bi = (&self.bi * factors[0]).quantize()?;

        q.h1_wt[0] = (&self.h1_wt[0] * factors[1]).quantize()?;
        q.h1_wt[1] = (&self.h1_wt[1] * factors[1]).quantize()?;

        // compare with a wt of 1 and this would be scaled by factors[0]
        q.h1_bi = (&self.h1_bi * (factors[0] * factors[1])).quantize()?;

        Ok(q)
    }
}

pub type Float = f32;

impl Network for Network768xH2<Float> {
    type Accumulators = (Vector<Float>, Vector<Float>); // white, black
    type Input = Float;
    type Output = Float;

    fn forward2(&self, pov: Color, (w, b): &Self::Accumulators) -> Self::Output {
        let mut output = self.h1_bi.get(0);
        match pov {
            Color::White => {
                w.apply_zip(&self.h1_wt[0], |x, y| output += crelu(*x) * *y);
                b.apply_zip(&self.h1_wt[1], |x, y| output += crelu(*x) * *y);
            }
            Color::Black => {
                b.apply_zip(&self.h1_wt[0], |x, y| output += crelu(*x) * *y);
                w.apply_zip(&self.h1_wt[1], |x, y| output += crelu(*x) * *y);
            }
        }
        output * 400.
    }

    fn new_accumulators(&self) -> Self::Accumulators {
        self.new_accumulators()
    }

    fn forward1_input(&self, wb: &mut Self::Accumulators, bd1: &Board, bd2: &Board) {
        self.forward1_input(wb, bd1, bd2)
    }

    fn forward1(&self, acc: &mut Self::Accumulators, b: &Board) {
        self.forward1(acc, b)
    }
}

#[inline(always)]
fn crelu_i16(x: i16) -> i32 {
    (x as i32).clamp(0, 255)
}

impl Network for Network768xH2<i16> {
    type Accumulators = (Vector<i16>, Vector<i16>); // white, black

    type Input = i16;
    type Output = i16;

    fn forward2(&self, pov: Color, (w, b): &Self::Accumulators) -> Self::Output {
        let mut output = self.h1_bi.get(0) as i32;
        match pov {
            Color::White => {
                w.apply_zip(&self.h1_wt[0], |x, y| output += crelu_i16(*x) * *y as i32);
                b.apply_zip(&self.h1_wt[1], |x, y| output += crelu_i16(*x) * *y as i32);
            }
            Color::Black => {
                b.apply_zip(&self.h1_wt[0], |x, y| output += crelu_i16(*x) * *y as i32);
                w.apply_zip(&self.h1_wt[1], |x, y| output += crelu_i16(*x) * *y as i32);
            }
        }
        output *= 400;
        output /= 255 * 64;
        output as i16
    }

    fn new_accumulators(&self) -> Self::Accumulators {
        self.new_accumulators()
    }

    fn forward1_input(&self, wb: &mut Self::Accumulators, bd1: &Board, bd2: &Board) {
        self.forward1_input(wb, bd1, bd2)
    }

    fn forward1(&self, acc: &mut Self::Accumulators, b: &Board) {
        self.forward1(acc, b)
    }
}

impl<T: Default + Copy> Network768xH2<T> {
    pub fn new(n_features: usize, n_hidden_layer: usize) -> Self {
        Self {
            n_features,
            n_hidden_layer,
            description: String::new(),
            wt: vec![Vector::new(n_hidden_layer); n_features],
            bi: Vector::new(n_hidden_layer),
            h1_wt: [Vector::new(n_hidden_layer), Vector::new(n_hidden_layer)],
            h1_bi: Vector::new(1),
        }
    }
}

impl<T: Copy + Debug + Default> Debug for Network768xH2<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Network768xH2")
            .field("n_features", &self.n_features)
            .field("n_hidden_layer", &self.n_hidden_layer)
            .finish()
    }
}

pub struct NetworkLoader;

impl NetworkLoader {
    pub fn read_postcard_format(buf: &[u8]) -> Result<Box<Network768xH2<i16>>> {
        let net: Network768xH2<i16> = postcard::from_bytes(buf)?;
        Ok(Box::new(net))
    }

    pub fn write_postcard_format<W: Write>(w: W, net: &Network768xH2<i16>) -> Result<()> {
        postcard::to_io(net, w)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;
    use std::io::Read as _;

    use odonata_base::catalog::Catalog;
    use odonata_base::infra::profiler::PerfProfiler;
    use odonata_base::infra::resources::relative_path;
    use test_log::test;
    use testresult::TestResult;

    use super::*;

    #[test]
    fn test_read_write_network() -> TestResult {
        let mut net = network_fixture();
        let file = fs_err::File::create(relative_path("ext/output/tmp/net.i16.postcard"))?;
        net.description = read_to_string(relative_path("ext/output/checkpoints/r61-description.txt"))?;
        NetworkLoader::write_postcard_format(&file, &net)?;
        drop(file);
        let mut file = fs_err::File::open(relative_path("ext/output/tmp/net.i16.postcard"))?;
        let mut buf = Vec::new();
        let _ = file.read_to_end(&mut buf)?;
        let net_pc = NetworkLoader::read_postcard_format(&buf)?;
        assert_eq!(net, net_pc);
        println!("{net_pc}");
        Ok(())
    }

    #[test]
    fn bench_forward() {
        let mut prof_fw1 = PerfProfiler::new("forward.forward1");
        let mut prof_fw2 = PerfProfiler::new("forward.forward2");
        let mut prof_abs = PerfProfiler::new("forward.abs");
        let mut prof_rel = PerfProfiler::new("forward.rel");
        let net = network_fixture();
        let bd1 = Catalog::example_game()[0].board();
        let bd2 = Catalog::example_game()[1].board();
        println!("{bd1}\n{bd2}");
        for (p, sq, c) in feature768_diff_iter(&bd1, &bd2) {
            println!("add {p} {sq} {c}");
        }
        for (p, sq, c) in feature768_diff_iter(&bd2, &bd1) {
            println!("sub {p} {sq} {c}");
        }

        let mut acc2 = net.new_accumulators();
        let sc2a = prof_abs.bench(|| {
            net.forward1(&mut acc2, &bd2);
            net.forward2(bd2.turn(), &acc2)
        });

        let mut acc1 = net.new_accumulators();
        net.forward1(&mut acc1, &bd1);
        let sc2b = prof_rel.bench(|| {
            net.forward1_input(&mut acc1, &bd1, &bd2);
            net.forward2(bd2.turn(), &acc1)
        });
        assert_eq!(sc2a, sc2b);
        prof_fw1.bench(|| {
            net.forward1(&mut acc1, &bd1);
        });
        prof_fw2.bench(|| net.forward2(bd1.turn(), &acc1));
    }

    fn network_fixture() -> Box<Network768xH2<i16>> {
        let file = "../../crates/odonata-engine/resources/r61-net.i16.bin";
        let mut buf = Vec::new();
        let _bytes = file_open(file).unwrap().read_to_end(&mut buf).unwrap();
        NetworkLoader::read_postcard_format(&buf).unwrap()
    }
}
