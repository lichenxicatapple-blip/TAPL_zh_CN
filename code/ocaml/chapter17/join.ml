type ty =
  | TyTop
  | TyBool
  | TyArr of ty * ty
  | TyRecord of (string * ty) list

type term =
  | TmTrue
  | TmFalse
  | TmRecord of (string * term) list
  | TmIf of term * term * term

exception Type_error of string

let rec subtype tyS tyT =
  tyS = tyT
  ||
  match (tyS, tyT) with
  | _, TyTop -> true
  | TyArr (tyS1, tyS2), TyArr (tyT1, tyT2) ->
      subtype tyT1 tyS1 && subtype tyS2 tyT2
  | TyRecord fS, TyRecord fT ->
      List.for_all
        (fun (label, tyT) ->
          match List.assoc_opt label fS with
          | Some tyS -> subtype tyS tyT
          | None -> false)
        fT
  | _, _ -> false

(* These mutually recursive functions are the solution printed for 17.3.1. *)
let rec join tyS tyT =
  match (tyS, tyT) with
  | TyArr (tyS1, tyS2), TyArr (tyT1, tyT2) -> (
      try TyArr (meet tyS1 tyT1, join tyS2 tyT2)
      with Not_found -> TyTop)
  | TyBool, TyBool -> TyBool
  | TyRecord fS, TyRecord fT ->
      let labelsS = List.map (fun (li, _) -> li) fS in
      let labelsT = List.map (fun (li, _) -> li) fT in
      let commonLabels =
        List.find_all (fun label -> List.mem label labelsT) labelsS
      in
      let commonFields =
        List.map
          (fun label ->
            let tySi = List.assoc label fS in
            let tyTi = List.assoc label fT in
            (label, join tySi tyTi))
          commonLabels
      in
      TyRecord commonFields
  | _ -> TyTop

and meet tyS tyT =
  match (tyS, tyT) with
  | _, TyTop -> tyS
  | TyTop, _ -> tyT
  | TyArr (tyS1, tyS2), TyArr (tyT1, tyT2) ->
      TyArr (join tyS1 tyT1, meet tyS2 tyT2)
  | TyBool, TyBool -> TyBool
  | TyRecord fS, TyRecord fT ->
      let labelsS = List.map (fun (li, _) -> li) fS in
      let labelsT = List.map (fun (li, _) -> li) fT in
      let allLabels =
        List.append labelsS
          (List.find_all (fun label -> not (List.mem label labelsS)) labelsT)
      in
      let allFields =
        List.map
          (fun label ->
            if not (List.mem label labelsS) then (label, List.assoc label fT)
            else if not (List.mem label labelsT) then
              (label, List.assoc label fS)
            else
              let tySi = List.assoc label fS in
              let tyTi = List.assoc label fT in
              (label, meet tySi tyTi))
          allLabels
      in
      TyRecord allFields
  | _ -> raise Not_found

(* This conditional branch is the remaining part of the printed solution. *)
let rec typeof = function
  | TmTrue | TmFalse -> TyBool
  | TmRecord fields ->
      TyRecord (List.map (fun (label, term) -> (label, typeof term)) fields)
  | TmIf (guard, then_branch, else_branch) ->
      if subtype (typeof guard) TyBool then
        join (typeof then_branch) (typeof else_branch)
      else raise (Type_error "guard of conditional not a boolean")

let () =
  let x_bool = TyRecord [ ("x", TyBool) ] in
  let xy_bool = TyRecord [ ("x", TyBool); ("y", TyBool) ] in
  let x_top = TyRecord [ ("x", TyTop) ] in
  assert (join xy_bool x_bool = x_bool);
  assert (join x_bool x_top = x_top);
  assert (meet x_bool x_top = x_bool);
  assert
    (meet x_bool (TyRecord [ ("y", TyBool) ])
    = TyRecord [ ("x", TyBool); ("y", TyBool) ]);
  assert
    (join
       (TyArr (TyTop, x_bool))
       (TyArr (TyBool, xy_bool))
    = TyArr (TyBool, x_bool));
  assert (join (TyArr (TyBool, TyBool)) (TyArr (x_bool, TyBool)) = TyTop);
  let branch_type =
    typeof
      (TmIf
         ( TmTrue,
           TmRecord [ ("x", TmTrue); ("y", TmFalse) ],
           TmRecord [ ("x", TmFalse) ] ))
  in
  assert (branch_type = x_bool);
  (try
     let _ = typeof (TmIf (TmRecord [], TmTrue, TmFalse)) in
     assert false
   with Type_error message ->
     assert (message = "guard of conditional not a boolean"));
  print_endline "chapter17 join/meet/conditional checker: ok"
