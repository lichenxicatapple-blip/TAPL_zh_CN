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

type target_ty =
  | TyUnit
  | TyTargetArr of target_ty * target_ty
  | TyTargetRecord of (string * target_ty) list

type target_term =
  | TmUnit
  | TmTargetRecord of (string * target_term) list
  | TmTargetProj of target_term * string
  | TmTargetVar of int
  | TmTargetAbs of target_ty * target_term
  | TmTargetApp of target_term * target_term

exception Translation_error of string

let rec translate_type = function
  | TyTop -> TyUnit
  | TyArr (parameter, result) ->
      TyTargetArr (translate_type parameter, translate_type result)
  | TyRecord fields ->
      TyTargetRecord
        (List.map
           (fun (label, field_type) -> (label, translate_type field_type))
           fields)

let rec coerce ty_s ty_t =
  if ty_s = ty_t then TmTargetAbs (translate_type ty_s, TmTargetVar 0)
  else
    match (ty_s, ty_t) with
    | _, TyTop -> TmTargetAbs (translate_type ty_s, TmUnit)
    | TyArr (s1, s2), TyArr (t1, t2) ->
        let parameter_coercion = coerce t1 s1 in
        let result_coercion = coerce s2 t2 in
        let coerced_argument =
          TmTargetApp (parameter_coercion, TmTargetVar 0)
        in
        let function_result =
          TmTargetApp (TmTargetVar 1, coerced_argument)
        in
        TmTargetAbs
          ( translate_type ty_s,
            TmTargetAbs
              ( translate_type t1,
                TmTargetApp (result_coercion, function_result) ) )
    | TyRecord fields_s, TyRecord fields_t ->
        let fields =
          List.map
            (fun (label, ty_ti) ->
              match List.assoc_opt label fields_s with
              | None ->
                  raise
                    (Translation_error ("missing source field " ^ label))
              | Some ty_si ->
                  ( label,
                    TmTargetApp
                      ( coerce ty_si ty_ti,
                        TmTargetProj (TmTargetVar 0, label) ) ))
            fields_t
        in
        TmTargetAbs (translate_type ty_s, TmTargetRecord fields)
    | _ -> raise (Translation_error "no subtype coercion exists")

let rec typeof context = function
  | TmRecord fields ->
      TyRecord
        (List.map (fun (label, field) -> (label, typeof context field)) fields)
  | TmProj (record, label) -> (
      match typeof context record with
      | TyRecord fields -> (
          match List.assoc_opt label fields with
          | Some ty -> ty
          | None -> raise (Translation_error ("missing field " ^ label)))
      | _ -> raise (Translation_error "projection expected a record"))
  | TmVar index -> (
      match List.nth_opt context index with
      | Some ty -> ty
      | None -> raise (Translation_error "unbound variable"))
  | TmAbs (parameter_type, body) ->
      TyArr (parameter_type, typeof (parameter_type :: context) body)
  | TmApp (fn, argument) -> (
      match typeof context fn with
      | TyArr (parameter_type, result_type) ->
          let argument_type = typeof context argument in
          let _ = coerce argument_type parameter_type in
          result_type
      | _ -> raise (Translation_error "application expected a function"))

let rec translate context term =
  match term with
  | TmRecord fields ->
      let translated =
        List.map
          (fun (label, field) ->
            let field_type, field_term = translate context field in
            ((label, field_type), (label, field_term)))
          fields
      in
      ( TyRecord (List.map fst translated),
        TmTargetRecord (List.map snd translated) )
  | TmProj (record, label) -> (
      let record_type, record_term = translate context record in
      match record_type with
      | TyRecord fields -> (
          match List.assoc_opt label fields with
          | Some field_type ->
              (field_type, TmTargetProj (record_term, label))
          | None -> raise (Translation_error ("missing field " ^ label)))
      | _ -> raise (Translation_error "projection expected a record"))
  | TmVar index -> (
      match List.nth_opt context index with
      | Some ty -> (ty, TmTargetVar index)
      | None -> raise (Translation_error "unbound variable"))
  | TmAbs (parameter_type, body) ->
      let body_type, body_term = translate (parameter_type :: context) body in
      ( TyArr (parameter_type, body_type),
        TmTargetAbs (translate_type parameter_type, body_term) )
  | TmApp (fn, argument) -> (
      let function_type, function_term = translate context fn in
      let argument_type, argument_term = translate context argument in
      match function_type with
      | TyArr (parameter_type, result_type) ->
          let argument_coercion = coerce argument_type parameter_type in
          ( result_type,
            TmTargetApp
              ( function_term,
                TmTargetApp (argument_coercion, argument_term) ) )
      | _ -> raise (Translation_error "application expected a function"))

let rec shift_walk distance cutoff = function
  | TmUnit -> TmUnit
  | TmTargetVar index ->
      if index >= cutoff then TmTargetVar (index + distance)
      else TmTargetVar index
  | TmTargetAbs (parameter_type, body) ->
      TmTargetAbs (parameter_type, shift_walk distance (cutoff + 1) body)
  | TmTargetApp (fn, argument) ->
      TmTargetApp
        (shift_walk distance cutoff fn, shift_walk distance cutoff argument)
  | TmTargetRecord fields ->
      TmTargetRecord
        (List.map
           (fun (label, field) ->
             (label, shift_walk distance cutoff field))
           fields)
  | TmTargetProj (record, label) ->
      TmTargetProj (shift_walk distance cutoff record, label)

let shift distance term = shift_walk distance 0 term

let rec substitute_walk variable replacement cutoff = function
  | TmUnit -> TmUnit
  | TmTargetVar index ->
      if index = variable + cutoff then shift cutoff replacement
      else TmTargetVar index
  | TmTargetAbs (parameter_type, body) ->
      TmTargetAbs
        ( parameter_type,
          substitute_walk variable replacement (cutoff + 1) body )
  | TmTargetApp (fn, argument) ->
      TmTargetApp
        ( substitute_walk variable replacement cutoff fn,
          substitute_walk variable replacement cutoff argument )
  | TmTargetRecord fields ->
      TmTargetRecord
        (List.map
           (fun (label, field) ->
             (label, substitute_walk variable replacement cutoff field))
           fields)
  | TmTargetProj (record, label) ->
      TmTargetProj
        (substitute_walk variable replacement cutoff record, label)

let substitute_top replacement body =
  shift (-1) (substitute_walk 0 (shift 1 replacement) 0 body)

let rec is_value = function
  | TmUnit | TmTargetAbs _ -> true
  | TmTargetRecord fields -> List.for_all (fun (_, t) -> is_value t) fields
  | _ -> false

let rec eval1 = function
  | TmTargetApp (TmTargetAbs (_, body), argument) when is_value argument ->
      Some (substitute_top argument body)
  | TmTargetApp (fn, argument) when not (is_value fn) ->
      Option.map (fun fn' -> TmTargetApp (fn', argument)) (eval1 fn)
  | TmTargetApp (fn, argument) when is_value fn ->
      Option.map (fun argument' -> TmTargetApp (fn, argument')) (eval1 argument)
  | TmTargetRecord fields ->
      let rec step_fields prefix = function
        | [] -> None
        | (label, field) :: rest when is_value field ->
            step_fields ((label, field) :: prefix) rest
        | (label, field) :: rest ->
            Option.map
              (fun field' ->
                TmTargetRecord
                  (List.rev_append prefix ((label, field') :: rest)))
              (eval1 field)
      in
      step_fields [] fields
  | TmTargetProj (TmTargetRecord fields, label)
    when List.for_all (fun (_, field) -> is_value field) fields ->
      List.assoc_opt label fields
  | TmTargetProj (record, label) ->
      Option.map (fun record' -> TmTargetProj (record', label)) (eval1 record)
  | _ -> None

let rec eval term = match eval1 term with Some next -> eval next | None -> term

let () =
  let source =
    TmApp
      ( TmAbs (TyRecord [ ("x", TyTop) ], TmProj (TmVar 0, "x")),
        TmRecord [ ("x", TmRecord []); ("y", TmRecord []) ] )
  in
  let source_type, translated = translate [] source in
  assert (source_type = TyTop);
  assert (eval translated = TmUnit);
  print_endline "chapter17 coercion translation: ok"
