CLASS zcl_rates DEFINITION PUBLIC.
  PUBLIC SECTION.
    INTERFACES zif_rates.
    METHODS reset.
    METHODS calculate_all.
    METHODS calculate IMPORTING iv_amount TYPE p iv_currency TYPE waers.
    TYPES: BEGIN OF ty_rate,
             currency TYPE waers,
             rate     TYPE p,
           END OF ty_rate.
    DATA mv_count TYPE i.
  PRIVATE SECTION.
    DATA mv_rate TYPE p.
ENDCLASS.
CLASS zcl_rates IMPLEMENTATION.
  METHOD reset.
    DATA lv_count TYPE i.
    CLEAR mv_rate.
  ENDMETHOD.
  METHOD calculate_all.
  ENDMETHOD.
  METHOD calculate.
    " no declarations here
    mv_rate = iv_amount.
  ENDMETHOD.
ENDCLASS.
