lo_service->read( EXPORTING iv_id         = lv_id
                            iv_lang       = sy-langu
                            iv_with_texts = abap_true
                  IMPORTING es_data       = ls_data
                            et_messages   = lt_messages ).
lo_service->read( EXPORTING iv_id   = lv_id
                  IMPORTING es_data = ls_data ).
lo_service->update( iv_id = lv_id ).
lo_service->check( EXPORTING iv_id = lv_id ).
lo_service->read( EXPORTING iv_identifier = lv_id
                  CHANGING  ct_log        = lt_log ).
lo_service->read( EXPORTING iv_id = lv_id
* iv_lang = sy-langu
                            iv_with_texts = abap_true
                  IMPORTING es_data       = ls_data ).
lo_service->read( EXPORTING
                            " the key
                            iv_id   = lv_id
                            iv_lang = sy-langu
                  " the result
                  IMPORTING es_data = ls_data
" no messages
).
