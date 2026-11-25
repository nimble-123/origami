CLASS lcl DEFINITION.
  PUBLIC SECTION.
    METHODS calculate
        IMPORTING
          iv_amount TYPE p
          iv_currency TYPE waers OPTIONAL
        RETURNING
          VALUE(rv_result) TYPE p
        RAISING
          zcx_calc_error.
    METHODS run.
    METHODS reset IMPORTING iv_hard TYPE abap_bool.
    METHODS e_with_a_quite_long_method_name
        IMPORTING
          iv_a_parameter_with_a_long_name TYPE zif_some_interface=>ty_some_long_type_name
          iv_b TYPE i.
    CLASS-METHODS create
        IMPORTING
          iv_a TYPE i DEFAULT 1
          iv_b TYPE string OPTIONAL
          iv_c TYPE REF TO zcl_x
        RAISING
          zcx_a
          zcx_b.
ENDCLASS.
