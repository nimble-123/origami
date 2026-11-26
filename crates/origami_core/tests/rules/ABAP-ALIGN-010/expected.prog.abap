READ TABLE lt_flights INTO ls_flight WITH KEY carrid = lv_carrid
                                              connid = lv_connid BINARY SEARCH.
READ TABLE lt_flights INTO ls_flight WITH KEY primary_key COMPONENTS carrid = lv_carrid
                                                                     connid = lv_connid.
READ TABLE lt_flights TRANSPORTING NO FIELDS WITH TABLE KEY carrid = lv_carrid
                                                            connid = lv_connid.
DELETE TABLE lt_flights WITH TABLE KEY carrid = lv_carrid
                                       connid = lv_connid.
READ TABLE lt_flights INTO ls_flight WITH KEY carrid = lv_carrid.
READ TABLE lt_flights INTO ls_flight WITH KEY (lv_name) = lv_carrid connid = lv_connid.
