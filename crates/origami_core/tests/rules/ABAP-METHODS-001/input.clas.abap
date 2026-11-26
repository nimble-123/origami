CLASS lcl DEFINITION.
  PUBLIC SECTION.
    METHODS calculate IMPORTING iv_amount TYPE p iv_currency TYPE waers OPTIONAL RETURNING VALUE(rv_result) TYPE p RAISING zcx_calc_error.
    METHODS run.
    METHODS reset IMPORTING iv_hard TYPE abap_bool.
    METHODS m ABSTRACT.
    CLASS-METHODS remove IMPORTING iv_tmp_key TYPE aufnr.
    METHODS get_name RETURNING VALUE(rv_name) TYPE string.
    METHODS test_it FOR TESTING RAISING cx_static_check.
    METHODS on_changed FOR EVENT changed OF zcl_x IMPORTING sender.
    METHODS: first IMPORTING iv_a TYPE i, second.
    METHODS precheck_create FOR PRECHECK IMPORTING entities FOR CREATE statuscommand.
    METHODS setdefaults FOR DETERMINE ON MODIFY IMPORTING keys FOR UserOrg~SetDefaults.
    METHODS get_global_authorizations FOR GLOBAL AUTHORIZATION IMPORTING REQUEST requested_authorizations FOR UserOrg RESULT result.
    METHODS get_instance_authorizations FOR INSTANCE AUTHORIZATION IMPORTING keys REQUEST requested_authorizations FOR UserOrg RESULT result.
    METHODS modify FOR MODIFY IMPORTING entities_c FOR CREATE travel entities_u FOR UPDATE travel.
    METHODS rba FOR READ IMPORTING keys_rba FOR READ travel\_booking FULL result_requested RESULT result LINK association_links.
    METHODS e_with_a_quite_long_method_name IMPORTING iv_a_parameter_with_a_long_name TYPE zif_some_interface=>ty_some_long_type_name iv_b TYPE i.
    CLASS-METHODS create IMPORTING iv_a TYPE i DEFAULT 1 iv_b TYPE string OPTIONAL iv_c TYPE REF TO zcl_x RAISING zcx_a zcx_b.
    METHODS read IMPORTING
      " the key
      iv_id TYPE i
      " the language
      iv_lang TYPE sy-langu OPTIONAL
* old: iv_text TYPE string
      EXPORTING es_data TYPE ty_data
      " last
      .
ENDCLASS.
