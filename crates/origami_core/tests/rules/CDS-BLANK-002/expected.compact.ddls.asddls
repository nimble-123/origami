@EndUserText.label: 'Travel'
define view entity ZI_Travel
  with parameters p_date : abap.dats
  as select from /dmo/travel as t
  inner join /dmo/agency as a on t.agency_id = a.agency_id
  association [0..1] to /DMO/I_Agency as _Agency on $projection.AgencyID = _Agency.AgencyID
{
  key t.travel_id as TravelID,
      @Semantics.amount.currencyCode: 'CurrencyCode'
      t.total_price as TotalPrice,
      t.agency_id as AgencyID,
      _Agency
}
where t.status <> 'X'
