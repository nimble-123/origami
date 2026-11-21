CLASS lcl DEFINITION.
  PUBLIC SECTION.
    METHODS calculate IMPORTING iv_amount TYPE p iv_currency TYPE waers OPTIONAL RETURNING VALUE(rv_result) TYPE p RAISING zcx_calc_error.
    METHODS run.
ENDCLASS.
