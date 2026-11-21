FORM read_flights
    TABLES
      et_flights STRUCTURE sflight
    USING
      iv_carrier TYPE s_carr_id
      iv_date TYPE s_date
    CHANGING
      cv_count TYPE i.
ENDFORM.
FORM short USING iv_a TYPE i.
ENDFORM.
