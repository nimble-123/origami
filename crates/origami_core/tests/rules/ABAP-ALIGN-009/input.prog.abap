CLEAR: lv_count, lt_items.
CLEAR: lv_single.
FREE: lt_a, lt_b.
SORT lt_flights BY carrid ASCENDING connid DESCENDING fldate AS TEXT.
SORT lt_flights STABLE BY carrid connid.
SORT lt_flights BY carrid.
SORT lt_flights BY (lv_name) connid.
TRY.
    book( ).
  CATCH cx_flight_full cx_flight_closed INTO DATA(lx_error).
  CATCH BEFORE UNWIND cx_a cx_b.
  CATCH cx_other.
ENDTRY.
