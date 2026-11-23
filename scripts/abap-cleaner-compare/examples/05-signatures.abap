class zcl_rates definition public final create public.
  public section.
    methods calculate importing iv_amount type p iv_currency type waers iv_date type d returning value(rv_result) type p raising zcx_calc_error.
    methods reset.
endclass.
