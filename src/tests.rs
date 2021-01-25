use crate::algebra::*;
use crate::util::VecMap;
use crate::{Instruction, ProofGF2P8, ConnectionInstruction};

use rand::{Rng, thread_rng};
use rand::RngCore;
use crate::algebra::gf2::{GF2P8, BitScalar};
use rand_core::OsRng;
use crate::preprocessing::Proof;
use async_std::task;

pub fn random_scalar<D: Domain, R: RngCore>(rng: &mut R) -> D::Scalar {
    let mut share = vec![D::Sharing::ZERO; D::Batch::DIMENSION];
    let mut batch = vec![D::Batch::ZERO; D::Sharing::DIMENSION];
    batch[0] = D::Batch::gen(rng);
    D::convert(&mut share[..], &mut batch[..]);
    share[0].get(0)
}

pub fn random_scalars<D: Domain, R: RngCore>(rng: &mut R, length: usize) -> Vec<D::Scalar> {
    let mut input = Vec::with_capacity(length);
    for _ in 0..length {
        input.push(random_scalar::<D, _>(rng))
    }
    input
}

// Evaluates a program (in the clear)
pub fn evaluate_program<D: Domain>(
    program: &[Instruction<D::Scalar>],
    inputs: &[D::Scalar],
    branch: &[D::Scalar],
) -> (Vec<usize>, Vec<D::Scalar>) {
    let mut wires = VecMap::new();
    let mut output = Vec::new();
    let mut output_wires = Vec::new();
    let mut inputs = inputs.iter().cloned();
    let mut branch = branch.iter().cloned();

    for step in program {
        match *step {
            Instruction::NrOfWires(nr) => {}
            Instruction::Input(dst) => {
                wires.set(dst, inputs.next().unwrap());
            }
            Instruction::Branch(dst) => {
                wires.set(dst, branch.next().unwrap());
            }
            Instruction::LocalOp(dst, src) => {
                wires.set(dst, wires.get(src).operation());
            }
            Instruction::Add(dst, src1, src2) => {
                wires.set(dst, wires.get(src1) + wires.get(src2));
            }
            Instruction::Mul(dst, src1, src2) => {
                wires.set(dst, wires.get(src1) * wires.get(src2));
            }
            Instruction::Const(dst, c) => {
                wires.set(dst, c);
            }
            Instruction::AddConst(dst, src, c) => {
                wires.set(dst, wires.get(src) + c);
            }
            Instruction::MulConst(dst, src, c) => {
                wires.set(dst, wires.get(src) * c);
            }
            Instruction::Output(src) => {
                output.push(wires.get(src));
                output_wires.push(src);
            }
        }
    }

    (output_wires, output)
}

// Evaluates two programs with fieldswitching (in the clear)
pub fn evaluate_fieldswitching_btoa_program<D: Domain, D2: Domain>(
    conn_program: &[ConnectionInstruction],
    program1: &[Instruction<D::Scalar>],
    program2: &[Instruction<D2::Scalar>],
    inputs1: &[D::Scalar],
    inputs2: &[D2::Scalar],
    branch1: &[D::Scalar],
    branch2: &[D2::Scalar],
) -> Vec<D::Scalar> {
    let (out_wires, output1) = evaluate_program::<D2>(program2, inputs2, branch2);

    let mut wires1 = Vec::new();


    for step in conn_program {
        match *step {
            ConnectionInstruction::BToA(dst, src) => {
                let mut input = D::Scalar::ZERO;
                let mut pow_two = D::Scalar::ONE;
                let two = D::Scalar::ONE + D::Scalar::ONE;
                for &_src in src.iter() {
                    let index = out_wires.iter().position(|&x| x == _src).unwrap();
                    input = input + convert_bit::<D2, D>(output1[index]) * pow_two;
                    pow_two = two * pow_two;
                }
                // wires1.set(dst, input);
                wires1.push(input); //TODO: change order
            }
            ConnectionInstruction::AToB(dst, src) => {
                // let mut output = output1[src].clone();
                // let mut pow_two = D::Scalar::ONE;
                // let two = D::Scalar::ONE + D::Scalar::ONE;
                // while output < pow_two {
                //     pow_two = two * pow_two;
                // }
                // for _dst in dst {
                //     pow_two = pow_two / two;
                //     if pow_two < output {
                //         output = output - pow_two;
                //         wires2.set(_dst, D2::Scalar::ONE);
                //     } else {
                //         wires2.set(_dst, D2::Scalar::ZERO);
                //     }
                // }
            }
            _ => {}
        }
    }

    let (_wires, output2) = evaluate_program::<D>(program1, &wires1[..], branch1);

    output2
}

fn convert_bit<D: Domain, D2: Domain>(input: D::Scalar) -> D2::Scalar {
    if input == D::Scalar::ONE {
        return D2::Scalar::ONE;
    } else {
        return D2::Scalar::ZERO;
    }
}

// Generates a random program for property based test
pub fn random_program<D: Domain, R: RngCore>(
    rng: &mut R,
    length: usize,
    memory: usize,
) -> (usize, usize, Vec<Instruction<D::Scalar>>) {
    let mut program: Vec<Instruction<D::Scalar>> = Vec::new();
    let mut assigned: Vec<usize> = vec![0];
    let mut num_inputs: usize = 1;
    let mut num_branch: usize = 0;

    program.push(Instruction::Input(0));

    while program.len() < length {
        // random source and destination indexes
        let dst: usize = rng.gen::<usize>() % memory;
        let src1: usize = assigned[rng.gen::<usize>() % assigned.len()];
        let src2: usize = assigned[rng.gen::<usize>() % assigned.len()];

        // pick random instruction
        match rng.gen::<usize>() % 8 {
            0 => {
                program.push(Instruction::Input(dst));
                assigned.push(dst);
                num_inputs += 1;
            }
            1 => {
                program.push(Instruction::Branch(dst));
                assigned.push(dst);
                num_branch += 1;
            }
            2 => {
                program.push(Instruction::Add(dst, src1, src2));
                assigned.push(dst);
            }
            3 => {
                program.push(Instruction::Mul(dst, src1, src2));
                assigned.push(dst);
            }
            4 => {
                program.push(Instruction::AddConst(dst, src1, random_scalar::<D, _>(rng)));
                assigned.push(dst);
            }
            5 => {
                program.push(Instruction::MulConst(dst, src1, random_scalar::<D, _>(rng)));
                assigned.push(dst);
            }
            6 => {
                program.push(Instruction::Output(src1));
            }
            7 => program.push(Instruction::LocalOp(dst, src1)),
            _ => unreachable!(),
        }
    }

    (num_inputs, num_branch, program)
}

#[test]
pub fn test_integration() {
    let mut rng = thread_rng();

    let program = mini_program::<GF2P8>();
    let input = random_scalars::<GF2P8, _>(&mut rng, 4);

    let branch: Vec<BitScalar> = vec![];
    let branches: Vec<Vec<BitScalar>> = vec![branch];

    let (_wires, output) = evaluate_program::<GF2P8>(&program[..], &input[..], &branches[0][..]);
    print!("{:?}", input);
    print!("{:?}", output);

    let proof = ProofGF2P8::new(None, program.clone(), branches.clone(), input, 0);

    // prove preprocessing
    // pick global random seed
    let mut seed: [u8; 32] = [0; 32];
    OsRng.fill_bytes(&mut seed);

    let branches2: Vec<&[BitScalar]> = branches.iter().map(|b| &b[..]).collect();

    let proof2 = Proof::<GF2P8>::new(seed, &branches2[..], program.iter().cloned());
    assert!(task::block_on(proof2.0.verify(&branches2[..], program.clone().into_iter())).is_some());

    let verifier_output = proof.verify(None, program.clone(), branches).unwrap();
    assert_eq!(verifier_output, output);
}

#[test]
pub fn test_evaluate_program() {
    let mut rng = thread_rng();

    let program1 = mini_program::<GF2P8>();
    let program2 = mini_program::<GF2P8>();
    let conn_program = connection_program();
    let input = random_scalars::<GF2P8, _>(&mut rng, 4);

    let branch: Vec<BitScalar> = vec![];
    let branches: Vec<Vec<BitScalar>> = vec![branch];

    let output = evaluate_fieldswitching_btoa_program::<GF2P8, GF2P8>(&conn_program[..], &program1[..], &program2[..], &input[..], &input[..], &branches[0][..], &branches[0][..]);
    println!("{:?}", input);
    println!("{:?}", output);
    assert_eq!(output, input);
}

pub fn mini_program<D: Domain>() -> Vec<Instruction<D::Scalar>> {
    let mut program: Vec<Instruction<D::Scalar>> = Vec::new();
    program.push(Instruction::NrOfWires(8));
    program.push(Instruction::Input(0));
    program.push(Instruction::Input(1));
    program.push(Instruction::Input(2));
    program.push(Instruction::Input(3));

    program.push(Instruction::AddConst(4, 0, D::Scalar::ONE));
    program.push(Instruction::AddConst(5, 1, D::Scalar::ONE));
    program.push(Instruction::AddConst(6, 2, D::Scalar::ONE));
    program.push(Instruction::AddConst(7, 3, D::Scalar::ONE));

    program.push(Instruction::Output(4));
    program.push(Instruction::Output(5));
    program.push(Instruction::Output(6));
    program.push(Instruction::Output(7));

    program
}

pub fn connection_program() -> Vec<ConnectionInstruction> {
    let mut program: Vec<ConnectionInstruction> = Vec::new();

    let src1: [usize; 1] = [4];
    let src4: [usize; 1] = [7];
    let src2: [usize; 1] = [5];
    let src3: [usize; 1] = [6];
    program.push(ConnectionInstruction::BToA(0, src1));
    program.push(ConnectionInstruction::BToA(1, src2));
    program.push(ConnectionInstruction::BToA(2, src3));
    program.push(ConnectionInstruction::BToA(3, src4));

    program
}
