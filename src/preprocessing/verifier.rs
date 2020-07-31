use super::*;

use crate::Instruction;

use rand_core::RngCore;

macro_rules! new_sharings {
    ( $shares:expr, $batches:expr, $rngs:expr, $omit:expr ) => {{
        for j in 0..N {
            if $omit != j {
                $batches[j] = D::Batch::gen(&mut $rngs[j]);
            }
        }
        D::convert($shares, &$batches[..]);
    }};
}

/// Implementation of pre-processing phase used by the prover during online execution
pub struct PreprocessingExecution<D: Domain> {
    // interpreter state
    masks: VecMap<D::Sharing>, //
    omitted: usize,            // omitted player
    players: Vec<Player>,

    // input mask state
    next_input: usize,
    share_input: Vec<D::Sharing>,

    // multiplication state
    share_g: Vec<D::Sharing>,
    share_a: Vec<D::Sharing>, // beta sharings (from input)
    share_b: Vec<D::Sharing>, // alpha sharings (from input)
}

impl<'a, D: Domain> PreprocessingExecution<D> {
    pub fn new(views: &[View], omitted: usize) -> Self {
        debug_assert!(omitted < D::PLAYERS);
        PreprocessingExecution {
            next_input: D::Batch::DIMENSION,
            share_input: vec![D::Sharing::ZERO; D::Batch::DIMENSION],
            players: views.iter().map(Player::new).collect(),
            omitted,
            share_g: vec![D::Sharing::ZERO; D::Batch::DIMENSION],
            share_a: Vec::with_capacity(D::Batch::DIMENSION),
            share_b: Vec::with_capacity(D::Batch::DIMENSION),
            masks: VecMap::new(),
        }
    }

    #[inline(always)]
    fn generate<I: Iterator<Item = D::Batch>>(
        &mut self,
        ab_gamma: &mut Vec<D::Sharing>,
        corrections: &mut I,
        batch_g: &[D::Batch],
        batch_a: &mut [D::Batch],
        batch_b: &mut [D::Batch],
        batch_c: &mut [D::Batch],
        batch_gab: &mut [D::Batch],
    ) -> Option<()> {
        // transpose sharings into per player batches
        D::convert_inv(&mut batch_a[..], &self.share_a[..]);
        D::convert_inv(&mut batch_b[..], &self.share_b[..]);
        self.share_a.clear();
        self.share_b.clear();

        // compute random c sharing and reconstruct a,b sharings
        for i in 0..D::PLAYERS {
            if i != self.omitted {
                // create sharing of product of masks
                batch_c[i] = D::Batch::gen(&mut self.players[i].beaver);
                if i == 0 {
                    // correct shares for player 0 (correction bits)
                    batch_c[0] = batch_c[0] + corrections.next()?;
                }

                // mask with gamma sharings
                batch_gab[i] = batch_c[i] + batch_g[i];
            }
        }

        // transpose into shares
        let start = ab_gamma.len();
        ab_gamma.resize(start + D::Batch::DIMENSION, D::Sharing::ZERO);
        D::convert(&mut ab_gamma[start..], &batch_gab);
        Some(())
    }

    pub fn process(
        &mut self,
        program: &[Instruction<D::Scalar>], // program slice
        corrections: &[D::Batch],           // player 0 corrections (if needed)
        masks: &mut Vec<D::Sharing>,        // resulting sharings consumed by online phase
        ab_gamma: &mut Vec<D::Sharing>,     // a * b + \gamma sharings for online phase
    ) -> Option<()> {
        debug_assert_eq!(self.share_a.len(), 0);
        debug_assert_eq!(self.share_b.len(), 0);

        let mut corrections = corrections.iter().cloned();

        let mut batch_g = vec![D::Batch::ZERO; D::PLAYERS];
        let mut batch_a = vec![D::Batch::ZERO; D::PLAYERS];
        let mut batch_b = vec![D::Batch::ZERO; D::PLAYERS];
        let mut batch_c = vec![D::Batch::ZERO; D::PLAYERS];
        let mut batch_m = vec![D::Batch::ZERO; D::PLAYERS];
        let mut batch_gab = vec![D::Batch::ZERO; D::PLAYERS];

        // execute pre-processing for program slice
        for step in program {
            debug_assert_eq!(self.share_a.len(), self.share_b.len());
            debug_assert_eq!(self.share_g.len(), D::Batch::DIMENSION);
            debug_assert_eq!(self.share_input.len(), D::Batch::DIMENSION);
            debug_assert_eq!(batch_g[self.omitted], D::Batch::ZERO);
            debug_assert!(self.share_a.len() < D::Batch::DIMENSION);
            debug_assert!(self.share_a.len() < D::Batch::DIMENSION);
            match *step {
                Instruction::Input(dst) => {
                    // check if need for new batch of input masks
                    if self.next_input == D::Batch::DIMENSION {
                        for j in 0..D::PLAYERS {
                            if j != self.omitted {
                                batch_m[j] = D::Batch::gen(&mut self.players[j].input);
                            }
                        }
                        debug_assert_eq!(batch_m[self.omitted], D::Batch::ZERO);
                        D::convert(&mut self.share_input[..], &batch_m);
                        self.next_input = 0;
                    }

                    // assign the next unused input share to the destination wire
                    let mask = self.share_input[self.next_input];
                    self.masks.set(dst, mask);
                    self.next_input += 1;
                }
                Instruction::AddConst(dst, src, _c) => {
                    // noop in pre-processing
                    self.masks.set(dst, self.masks.get(src));
                }
                Instruction::MulConst(dst, src, c) => {
                    // resolve input
                    let sw = self.masks.get(src);

                    // let the single element act on the vector
                    self.masks.set(dst, sw.action(c));
                }
                Instruction::Add(dst, src1, src2) => {
                    // resolve inputs
                    let sw1 = self.masks.get(src1);
                    let sw2 = self.masks.get(src2);

                    // compute the sum and set output wire
                    self.masks.set(dst, sw1 + sw2);
                }
                Instruction::Mul(dst, src1, src2) => {
                    let next_idx = self.share_a.len();
                    if next_idx == 0 {
                        for j in 0..D::PLAYERS {
                            if j != self.omitted {
                                batch_g[j] = D::Batch::gen(&mut self.players[j].beaver);
                            }
                        }
                        debug_assert_eq!(batch_g[self.omitted], D::Batch::ZERO);
                        D::convert(&mut self.share_g[..], &batch_g);
                    }

                    // push the masks to the Beaver stack
                    let mask_a = self.masks.get(src1);
                    let mask_b = self.masks.get(src2);
                    let mask_g = self.share_g[next_idx];

                    // add input masks to the next multiplication batch
                    self.share_a.push(mask_a);
                    self.share_b.push(mask_b);

                    // assign mask to output
                    self.masks.set(dst, mask_g);

                    // return input masks to online phase
                    masks.push(mask_a);
                    masks.push(mask_b);

                    // if the batch is full, stop.
                    if self.share_a.len() == D::Batch::DIMENSION {
                        self.generate(
                            ab_gamma,
                            &mut corrections,
                            &batch_g,
                            &mut batch_a,
                            &mut batch_b,
                            &mut batch_c,
                            &mut batch_gab,
                        )?;
                    }
                }
                Instruction::Output(src) => {
                    // return to online phase for reconstruction of masked wire
                    masks.push(self.masks.get(src));
                }
            }
        }

        // pad with dummy values and compute last batch
        if self.share_a.len() > 0 {
            self.share_a.resize(D::Batch::DIMENSION, D::Sharing::ZERO);
            self.share_b.resize(D::Batch::DIMENSION, D::Sharing::ZERO);
            self.generate(
                ab_gamma,
                &mut corrections,
                &batch_g,
                &mut batch_a,
                &mut batch_b,
                &mut batch_c,
                &mut batch_gab,
            )
        } else {
            Some(())
        }
    }
}
