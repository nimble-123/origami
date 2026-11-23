CLASS lcl_flights DEFINITION FOR TESTING RISK LEVEL HARMLESS DURATION SHORT.
  PUBLIC SECTION.
    INTERFACES zif_reader.
    ALIASES read FOR zif_reader~read.
    ALIASES read_all_entries FOR zif_reader~read_all_entries.
    "! Reads one flight
    "! @parameter iv_carrier | Airline
    "! @parameter iv_connection_number | Flight number
    "! @raising zcx_not_found | No such flight
    METHODS get_flight IMPORTING iv_carrier TYPE s_carr_id iv_connection_number TYPE s_conn_id RAISING zcx_not_found.
    METHODS get_name REDEFINITION.
    METHODS get_description REDEFINITION.
  PRIVATE SECTION.
    METHODS reads_one_flight FOR TESTING.
    METHODS fails_without_carrier FOR TESTING RAISING cx_static_check.
ENDCLASS.

CLASS lcl_flights IMPLEMENTATION.
  METHOD reads_one_flight.
    ls_flight-carrid = 'LH' .
    ls_flight-connid = '0400' .
    ls_flight-seats_occupied = 120 .
    SELECT carrid, connid, cityfrom AS departure, cityto AS arrival FROM spfli INTO TABLE @DATA(lt_routes) WHERE carrid = @ls_flight-carrid.
    DATA(lv_status) = COND #( WHEN ls_flight-seats_occupied = 0 THEN 'empty' WHEN ls_flight-seats_occupied < 200 THEN 'open' ELSE 'full' ).
  ENDMETHOD.
ENDCLASS.
