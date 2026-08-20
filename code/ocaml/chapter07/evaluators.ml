type info = unit

type term =
  | TmVar of info * int * int
  | TmAbs of info * string * term
  | TmApp of info * term * term

exception NoRuleApplies

let termShift distance term =
  let rec walk cutoff = function
    | TmVar (fi, index, context_length) ->
        if index >= cutoff then
          TmVar (fi, index + distance, context_length + distance)
        else TmVar (fi, index, context_length + distance)
    | TmAbs (fi, name, body) ->
        TmAbs (fi, name, walk (cutoff + 1) body)
    | TmApp (fi, function_term, argument) ->
        TmApp (fi, walk cutoff function_term, walk cutoff argument)
  in
  walk 0 term

let termSubst variable replacement term =
  let rec walk cutoff = function
    | TmVar (fi, index, context_length) ->
        if index = variable + cutoff then termShift cutoff replacement
        else TmVar (fi, index, context_length)
    | TmAbs (fi, name, body) ->
        TmAbs (fi, name, walk (cutoff + 1) body)
    | TmApp (fi, function_term, argument) ->
        TmApp (fi, walk cutoff function_term, walk cutoff argument)
  in
  walk 0 term

let termSubstTop replacement body =
  termShift (-1) (termSubst 0 (termShift 1 replacement) body)

let isval _ = function TmAbs _ -> true | _ -> false

let rec eval1 context = function
  | TmApp (_, TmAbs (_, _, body), argument) when isval context argument ->
      termSubstTop argument body
  | TmApp (fi, function_value, argument) when isval context function_value ->
      TmApp (fi, function_value, eval1 context argument)
  | TmApp (fi, function_term, argument) ->
      TmApp (fi, eval1 context function_term, argument)
  | _ -> raise NoRuleApplies

let rec eval_small context term =
  try eval_small context (eval1 context term) with NoRuleApplies -> term

(* The big-step evaluator in the translator's solution to 7.3.1. *)
let rec eval_big context term =
  match term with
  | TmAbs _ -> term
  | TmApp (_, function_term, argument) ->
      let function_value = eval_big context function_term in
      let argument_value = eval_big context argument in
      (match function_value with
      | TmAbs (_, _, body) ->
          eval_big context (termSubstTop argument_value body)
      | _ -> raise NoRuleApplies)
  | _ -> raise NoRuleApplies

let expect_no_rule thunk =
  try
    let _ = thunk () in
    assert false
  with NoRuleApplies -> ()

let () =
  let identity = TmAbs ((), "x", TmVar ((), 0, 1)) in
  let constant =
    TmAbs ((), "x", TmAbs ((), "y", TmVar ((), 1, 2)))
  in
  let self_application =
    TmAbs
      ((), "x", TmApp ((), TmVar ((), 0, 1), TmVar ((), 0, 1)))
  in
  let examples =
    [
      TmApp ((), identity, identity);
      TmApp ((), TmApp ((), constant, identity), self_application);
    ]
  in
  List.iter (fun term -> assert (eval_small [] term = eval_big [] term)) examples;
  assert (eval_big [] (TmApp ((), identity, self_application)) = self_application);
  expect_no_rule (fun () -> eval_big [] (TmVar ((), 0, 1)));
  print_endline "chapter07 OCaml evaluators: ok"
