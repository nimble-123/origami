CLASS lcl_demo IMPLEMENTATION.
  METHOD run.
    ro_instance->mv_name  = iv_name.
    ro_instance->mv_id    = iv_id.
    ro_instance->mv_price = iv_price.

    ls_struc-component       = 1.
    ls_struc-comp2           = 'a'.
    ls_struc-start_date+6(2) = 31.
    ls_struc-c3              = VALUE #( ).

    <ls_field_symbol>-component = 1.
    <ls_field_symbol>-comp2     = 'a'.

    " alignment across comments and empty lines:
    ls_struc-component += 1.
    " comment
    ls_struc-comp2      = 'a'.

    ls_struc-c3         = VALUE #( ).

    " comment
    ls_struc-comp4     -= 1.

    ls_other-a = 1.
    lv_plain = 2.
    ls_other-bbb = 3.
    ls_third-ccc = 4.
    lt_table[ 1 ]-a = 5.
    lt_table[ 1 ]-bbbb = 6.

    ls_wide-a = 'This literal makes this line too wide when the group is aligned: the group keeps its single spaces'.
    ls_wide-bbbbb = 1.
  ENDMETHOD.
ENDCLASS.
