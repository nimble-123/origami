CLASS zcl_doc DEFINITION PUBLIC FINAL CREATE PUBLIC.
  PUBLIC SECTION.
    "! Reads a flight
    "!
    "! @parameter iv_carrier       | Airline
    "! @parameter iv_connection_id | Connection
    "! The connection must exist.
    "! @parameter rs_flight        | Flight
    "! @raising   zcx_not_found    | Flight not found
    METHODS read
      IMPORTING
        iv_carrier TYPE s_carr_id
        iv_connection_id TYPE s_conn_id
      RETURNING
        VALUE(rs_flight) TYPE sflight
      RAISING
        zcx_not_found.

    "! @parameter iv_id |
    "! @parameter iv_name_of_the_thing Name
    "! @exception not_found | Not found
    "! @parameter iv_flag   | Flag
    METHODS check IMPORTING iv_id TYPE i iv_name_of_the_thing TYPE string iv_flag TYPE abap_bool EXCEPTIONS not_found.

    "! @parameter iv_a | <p class="shorttext synchronized" lang="en">A parameter with a long description text</p>
    "! @parameter iv_parameter_with_a_long_name | <p class="shorttext synchronized" lang="en">Long name</p>
    METHODS wide IMPORTING iv_a TYPE i iv_parameter_with_a_long_name TYPE i.
ENDCLASS.

CLASS zcl_doc IMPLEMENTATION.
ENDCLASS.
