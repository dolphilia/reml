(* parser_flags.ml — Parser runtime feature switches *)

exception Experimental_effects_disabled of Lexing.position * Lexing.position

let experimental_effects_enabled_ref = ref false

let set_experimental_effects_enabled flag =
  experimental_effects_enabled_ref := flag

let experimental_effects_enabled () = !experimental_effects_enabled_ref
