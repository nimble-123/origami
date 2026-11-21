IF lv_a = 1.
do_a( ).
ELSE.
do_b( ).
ENDIF.
CASE lv_kind.
WHEN 'A'.
do_a( ).
WHEN OTHERS.
do_b( ).
ENDCASE.
TRY.
do_a( ).
CATCH cx_root INTO DATA(lx).
handle( lx ).
ENDTRY.
