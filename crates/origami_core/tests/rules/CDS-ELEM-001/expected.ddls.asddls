define view entity ZI_X
  as select from t

{
  key id,

      case currency
        when 'EUR' then 'X'
        when 'USD' then 'Y'
        else 'Z'
      end as CurrencyGroup,

      case flag when 'X' then 1 else 0 end as IsSet
}
