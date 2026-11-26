CALL FUNCTION 'Z_FLIGHT_TRAVEL_CREATE'
    EXPORTING is_travel  = ls_travel
              it_booking = lt_booking
    IMPORTING es_travel   = ls_result
              et_messages = lt_messages
    EXCEPTIONS not_found = 1
               OTHERS    = 2.
CALL FUNCTION 'POPUP_TO_CONFIRM'
    EXPORTING
*     TITLEBAR              = ' '
              text_question = lv_q
              text_button_1 = 'Ja'
*     ICON_BUTTON_1         = ' '
              display_cancel_button = abap_false
    IMPORTING answer = lv_answer.
*   EXCEPTIONS
*     TEXT_NOT_FOUND        = 1
CALL METHOD lo_service->read
    EXPORTING
              " the key
              iv_id = lv_id
              " the language
              iv_lang = sy-langu
    IMPORTING es_data = ls_data.
" no messages
CALL FUNCTION 'SCMS_STRING_TO_XSTRING'
    EXPORTING text = lv_text
*     MIMETYPE = ' '
    IMPORTING buffer = lv_buffer.
