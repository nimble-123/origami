lo_service->read( EXPORTING iv_id         = lv_id
                            iv_lang       = sy-langu
                            iv_with_texts = abap_true
                  IMPORTING es_data     = ls_data
                            et_messages = lt_messages ).
