define view entity ZI_Converted as select from ZI_Source {
  key Id,
  currency_conversion( amount => Amount, source_currency => Currency, target_currency => $parameters.p_currency, exchange_rate_date => $parameters.p_date ) as Converted,
  abs( Amount ) as AbsAmount,
  round( Amount, 2 ) as Rounded
}
