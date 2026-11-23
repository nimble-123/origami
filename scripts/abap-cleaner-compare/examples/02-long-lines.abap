if lv_amount > 0 and lv_currency is not initial and ls_data-status = 'A' and ls_data-created_by = sy-uname and lv_flag = abap_true.
  book( ).
endif.
select carrid, connid, fldate from sflight where carrid = @lv_carrier and fldate >= @sy-datum into table @data(lt_flights).
