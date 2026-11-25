define view entity ZI_Booking with parameters p_date : abap.dats, p_currency : abap.cuky as select from ZI_Source( p_date : $parameters.p_date, p_currency : $parameters.p_currency ) as Source {
  key Source.id as Id
}
