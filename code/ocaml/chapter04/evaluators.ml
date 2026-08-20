type info = unit

let dummyinfo = ()

type term =
  | TmTrue of info
  | TmFalse of info
  | TmIf of info * term * term * term
  | TmZero of info
  | TmSucc of info * term
  | TmPred of info * term
  | TmIsZero of info * term

exception NoRuleApplies

let rec isnumericval = function
  | TmZero _ -> true
  | TmSucc (_, t1) -> isnumericval t1
  | _ -> false

let rec eval1 = function
  | TmIf (_, TmTrue _, t2, _) -> t2
  | TmIf (_, TmFalse _, _, t3) -> t3
  | TmIf (fi, t1, t2, t3) -> TmIf (fi, eval1 t1, t2, t3)
  | TmSucc (fi, t1) -> TmSucc (fi, eval1 t1)
  | TmPred (_, TmZero _) -> TmZero dummyinfo
  | TmPred (_, TmSucc (_, nv1)) when isnumericval nv1 -> nv1
  | TmPred (fi, t1) -> TmPred (fi, eval1 t1)
  | TmIsZero (_, TmZero _) -> TmTrue dummyinfo
  | TmIsZero (_, TmSucc (_, nv1)) when isnumericval nv1 -> TmFalse dummyinfo
  | TmIsZero (fi, t1) -> TmIsZero (fi, eval1 t1)
  | _ -> raise NoRuleApplies

(* The tail-recursive formulation in the author's solution to 4.2.1. *)
let rec eval_small t =
  let next = try Some (eval1 t) with NoRuleApplies -> None in
  match next with Some t' -> eval_small t' | None -> t

(* The big-step evaluator in the translator's solution to 4.2.2. *)
let rec eval_big t =
  match t with
  | TmTrue _ | TmFalse _ | TmZero _ -> t
  | TmIf (_, t1, t2, t3) -> (
      match eval_big t1 with
      | TmTrue _ -> eval_big t2
      | TmFalse _ -> eval_big t3
      | _ -> raise NoRuleApplies)
  | TmSucc (fi, t1) ->
      let v1 = eval_big t1 in
      if isnumericval v1 then TmSucc (fi, v1) else raise NoRuleApplies
  | TmPred (_, t1) -> (
      match eval_big t1 with
      | TmZero _ -> TmZero dummyinfo
      | TmSucc (_, nv1) when isnumericval nv1 -> nv1
      | _ -> raise NoRuleApplies)
  | TmIsZero (_, t1) -> (
      match eval_big t1 with
      | TmZero _ -> TmTrue dummyinfo
      | TmSucc (_, nv1) when isnumericval nv1 -> TmFalse dummyinfo
      | _ -> raise NoRuleApplies)

let expect_no_rule thunk =
  try
    let _ = thunk () in
    assert false
  with NoRuleApplies -> ()

let () =
  let zero = TmZero () in
  let one = TmSucc ((), zero) in
  let two = TmSucc ((), one) in
  let examples =
    [
      TmPred ((), two);
      TmIsZero ((), TmPred ((), one));
      TmIf ((), TmFalse (), TmPred ((), TmTrue ()), two);
      TmSucc ((), TmPred ((), two));
    ]
  in
  List.iter (fun term -> assert (eval_small term = eval_big term)) examples;
  assert (eval_big (TmPred ((), two)) = one);
  assert
    (eval_big (TmIf ((), TmFalse (), TmPred ((), TmTrue ()), two)) = two);
  expect_no_rule (fun () -> eval_big (TmSucc ((), TmTrue ())));
  print_endline "chapter04 OCaml evaluators: ok"
