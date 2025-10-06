# decl レコード
s/{ attrs; vis; kind; span }/{ decl_attrs = attrs; decl_vis = vis; decl_kind = kind; decl_span = span }/g
# attribute レコード
s/{ name; args; attr_span = span }/{ attr_name = name; attr_args = args; attr_span = span }/g
# fn_decl レコード (複数行にまたがるため個別に対応が必要)
# match_arm レコード
s/{ pattern = pat; guard; body; arm_span = span }/{ arm_pattern = pat; arm_guard = guard; arm_body = body; arm_span = span }/g
s/{ pattern; guard; body; arm_span }/{ arm_pattern = pattern; arm_guard = guard; arm_body = body; arm_span }/g
