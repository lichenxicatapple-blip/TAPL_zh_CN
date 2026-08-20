type info = unit

type ty =
  | TyRecord of (string * ty) list
  | TyTop
  | TyArr of ty * ty

type term =
  | TmRecord of info * (string * term) list
  | TmProj of info * term * string
  | TmVar of info * int * int
  | TmAbs of info * string * ty * term
  | TmApp of info * term * term

type binding = VarBind of ty
type context = (string * binding) list

exception Type_error of string

let error _ message = raise (Type_error message)

let addbinding context name binding = (name, binding) :: context

let getTypeFromContext fi context index =
  match List.nth_opt context index with
  | Some (_, VarBind ty) -> ty
  | None -> error fi "variable lookup failure"

(* This is the subtype checker printed in Section 17.2. *)
let rec subtype tyS tyT =
  (=) tyS tyT
  ||
  match (tyS, tyT) with
  | TyRecord fS, TyRecord fT ->
      List.for_all
        (fun (li, tyTi) ->
          try
            let tySi = List.assoc li fS in
            subtype tySi tyTi
          with Not_found -> false)
        fT
  | _, TyTop -> true
  | TyArr (tyS1, tyS2), TyArr (tyT1, tyT2) ->
      subtype tyT1 tyS1 && subtype tyS2 tyT2
  | _, _ -> false

(* This is the type checker printed in Section 17.3. *)
let rec typeof ctx t =
  match t with
  | TmRecord (_, fields) ->
      let fieldtys =
        List.map (fun (li, ti) -> (li, typeof ctx ti)) fields
      in
      TyRecord fieldtys
  | TmProj (fi, t1, label) -> (
      match typeof ctx t1 with
      | TyRecord fieldtys -> (
          try List.assoc label fieldtys
          with Not_found -> error fi ("label " ^ label ^ " not found"))
      | _ -> error fi "Expected record type")
  | TmVar (fi, index, _) -> getTypeFromContext fi ctx index
  | TmAbs (_, name, tyT1, body) ->
      let ctx' = addbinding ctx name (VarBind tyT1) in
      let tyT2 = typeof ctx' body in
      TyArr (tyT1, tyT2)
  | TmApp (fi, t1, t2) -> (
      let tyT1 = typeof ctx t1 in
      let tyT2 = typeof ctx t2 in
      match tyT1 with
      | TyArr (tyT11, tyT12) ->
          if subtype tyT2 tyT11 then tyT12
          else error fi "parameter type mismatch"
      | _ -> error fi "arrow type expected")

let expect_type_error message thunk =
  try
    let _ = thunk () in
    assert false
  with Type_error actual -> assert (actual = message)

let () =
  let empty_record = TyRecord [] in
  let xy_record = TyRecord [ ("x", TyTop); ("y", empty_record) ] in
  let x_record = TyRecord [ ("x", TyTop) ] in
  assert (subtype xy_record x_record);
  assert (not (subtype x_record xy_record));
  assert
    (subtype
       (TyArr (TyTop, x_record))
       (TyArr (xy_record, TyTop)));
  let record_term =
    TmRecord
      ((), [ ("x", TmRecord ((), [])); ("y", TmRecord ((), [])) ])
  in
  assert (typeof [] record_term = TyRecord [ ("x", empty_record); ("y", empty_record) ]);
  assert (typeof [] (TmProj ((), record_term, "x")) = empty_record);
  let accepts_x =
    TmAbs ((), "r", x_record, TmProj ((), TmVar ((), 0, 1), "x"))
  in
  assert (typeof [] (TmApp ((), accepts_x, record_term)) = TyTop);
  expect_type_error "label missing not found" (fun () ->
      typeof [] (TmProj ((), record_term, "missing")));
  expect_type_error "parameter type mismatch" (fun () ->
      typeof []
        (TmApp
           ((), TmAbs ((), "r", xy_record, TmVar ((), 0, 1)),
            TmRecord ((), [ ("x", TmRecord ((), [])) ]))));
  expect_type_error "arrow type expected" (fun () ->
      typeof [] (TmApp ((), record_term, record_term)));
  print_endline "chapter17 core subtype/type checker: ok"
