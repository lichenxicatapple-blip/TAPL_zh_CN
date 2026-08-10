type ty =
  | TyTop
  | TyArr of ty * ty
  | TyRecord of (string * ty) list

type term =
  | TmRecord of (string * term) list
  | TmProj of term * string
  | TmVar of int
  | TmAbs of ty * term
  | TmApp of term * term

type subtype_error =
  | MissingField of string list * string * ty * ty
  | ShapeMismatch of string list * ty * ty

exception Subtype_error of subtype_error

type type_error =
  | UnboundVariable of int * int
  | MissingProjectionField of string * ty
  | ProjectionFromNonRecord of ty
  | SubtypingFailed of subtype_error
  | ExpectedFunction of ty

exception Type_error of type_error

let rec check_subtype path ty_s ty_t =
  if ty_s = ty_t then ()
  else
    match (ty_s, ty_t) with
    | _, TyTop -> ()
    | TyArr (s1, s2), TyArr (t1, t2) ->
        check_subtype ("parameter" :: path) t1 s1;
        check_subtype ("result" :: path) s2 t2
    | TyRecord fields_s, TyRecord fields_t ->
        List.iter
          (fun (label, ty_ti) ->
            match List.assoc_opt label fields_s with
            | None ->
                raise
                  (Subtype_error
                     (MissingField (List.rev path, label, ty_s, ty_t)))
            | Some ty_si -> check_subtype (label :: path) ty_si ty_ti)
          fields_t
    | _ ->
        raise
          (Subtype_error (ShapeMismatch (List.rev path, ty_s, ty_t)))

let rec typeof context term =
  match term with
  | TmRecord fields ->
      TyRecord
        (List.map (fun (label, field) -> (label, typeof context field)) fields)
  | TmProj (record, label) -> (
      match typeof context record with
      | TyRecord fields -> (
          match List.assoc_opt label fields with
          | Some field_type -> field_type
          | None ->
              raise (Type_error (MissingProjectionField (label, TyRecord fields))))
      | other -> raise (Type_error (ProjectionFromNonRecord other)))
  | TmVar index -> (
      match List.nth_opt context index with
      | Some ty -> ty
      | None -> raise (Type_error (UnboundVariable (index, List.length context))))
  | TmAbs (parameter_type, body) ->
      TyArr (parameter_type, typeof (parameter_type :: context) body)
  | TmApp (fn, argument) ->
      let function_type = typeof context fn in
      let argument_type = typeof context argument in
      (match function_type with
      | TyArr (parameter_type, result_type) ->
          (try
             check_subtype [] argument_type parameter_type;
             result_type
           with Subtype_error detail ->
             raise (Type_error (SubtypingFailed detail)))
      | other -> raise (Type_error (ExpectedFunction other)))

let string_of_path path =
  match path with [] -> "<root>" | labels -> String.concat "." labels

let rec string_of_ty = function
  | TyTop -> "Top"
  | TyArr (input, output) ->
      Printf.sprintf "(%s -> %s)" (string_of_ty input) (string_of_ty output)
  | TyRecord fields ->
      let field (label, ty) = Printf.sprintf "%s: %s" label (string_of_ty ty) in
      Printf.sprintf "{%s}" (String.concat ", " (List.map field fields))

let string_of_type_error = function
  | UnboundVariable (index, length) ->
      Printf.sprintf "variable %d is outside a context of length %d" index length
  | MissingProjectionField (label, record) ->
      Printf.sprintf "record type %s has no field %s" (string_of_ty record) label
  | ProjectionFromNonRecord actual ->
      Printf.sprintf "projection expected a record, but found %s"
        (string_of_ty actual)
  | ExpectedFunction actual ->
      Printf.sprintf "application expected a function, but found %s"
        (string_of_ty actual)
  | SubtypingFailed (MissingField (path, label, actual, expected)) ->
      Printf.sprintf
        "below %s: actual type %s lacks field %s required by %s"
        (string_of_path path) (string_of_ty actual) label (string_of_ty expected)
  | SubtypingFailed (ShapeMismatch (path, actual, expected)) ->
      Printf.sprintf
        "below %s: actual type %s is incompatible with expected type %s"
        (string_of_path path) (string_of_ty actual) (string_of_ty expected)

let expect_type_error predicate thunk =
  try
    let _ = thunk () in
    assert false
  with Type_error error -> assert (predicate error)

let () =
  let nested_source =
    TyRecord [ ("payload", TyRecord [ ("x", TyTop); ("y", TyTop) ]) ]
  in
  let nested_target = TyRecord [ ("payload", TyRecord [ ("x", TyTop) ]) ] in
  check_subtype [] nested_source nested_target;
  expect_type_error
    (function
      | SubtypingFailed (MissingField ([ "payload" ], "missing", _, _)) -> true
      | _ -> false)
    (fun () ->
      typeof []
        (TmApp
           ( TmAbs
               ( TyRecord [ ("payload", TyRecord [ ("missing", TyTop) ]) ],
                 TmVar 0 ),
             TmRecord
               [
                 ( "payload",
                   TmRecord [ ("x", TmRecord []); ("y", TmRecord []) ] );
               ] )));
  expect_type_error
    (function
      | SubtypingFailed (ShapeMismatch ([ "payload" ], _, _)) -> true
      | _ -> false)
    (fun () ->
      typeof []
        (TmApp
           ( TmAbs
               ( TyRecord [ ("payload", TyArr (TyTop, TyTop)) ],
                 TmVar 0 ),
             TmRecord [ ("payload", TmRecord []) ] )));
  expect_type_error
    (function UnboundVariable (2, 0) -> true | _ -> false)
    (fun () -> typeof [] (TmVar 2));
  expect_type_error
    (fun error ->
      string_of_type_error error = "record type {} has no field missing")
    (fun () -> typeof [] (TmProj (TmRecord [], "missing")));
  expect_type_error
    (fun error ->
      string_of_type_error error
      = "projection expected a record, but found (Top -> Top)")
    (fun () -> typeof [] (TmProj (TmAbs (TyTop, TmVar 0), "x")));
  expect_type_error
    (fun error ->
      string_of_type_error error = "application expected a function, but found {}")
    (fun () -> typeof [] (TmApp (TmRecord [], TmRecord [])));
  let formatted_mismatch =
    try
      check_subtype [ "payload" ] (TyRecord []) (TyArr (TyTop, TyTop));
      assert false
    with Subtype_error detail -> string_of_type_error (SubtypingFailed detail)
  in
  assert
    (formatted_mismatch
    = "below payload: actual type {} is incompatible with expected type (Top -> Top)");
  print_endline "chapter17 diagnostics: ok"
