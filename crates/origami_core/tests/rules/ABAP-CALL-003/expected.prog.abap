CALL FUNCTION 'Z_FLIGHT_TRAVEL_CREATE'
  EXPORTING  is_travel   = ls_travel
             it_booking  = lt_booking
  IMPORTING  es_travel   = ls_result
             et_messages = lt_messages
  EXCEPTIONS not_found   = 1
             OTHERS      = 2.
