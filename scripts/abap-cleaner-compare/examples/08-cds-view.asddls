@AccessControl.authorizationCheck: #NOT_REQUIRED
@EndUserText.label: 'Travel'
define view entity ZI_Travel as select from /dmo/travel association [0..1] to /DMO/I_Agency as _Agency on $projection.AgencyID = _Agency.AgencyID {
  key travel_id as TravelID, agency_id as AgencyID, customer_id as CustomerID,
  _Agency
}
