DATA lv_count TYPE i.

lv_count = lines( mt_items ) + lvOffset.
IF lv_count > 0 AND mv_flag = abap_true.
  me->mv_total = |{ lv_count }| && 'TEXT'.
ENDIF.
