import { respond } from '../util'

const getZeroTierMember = async address => {
  const apiToken = await SETTINGS.get('zerotier_central_api_token')
  const networkId = await SETTINGS.get('zerotier_network_id')

  return fetch(`https://my.zerotier.com/api/network/${networkId}/member/${address}`, {
    headers: { authorization: `Bearer ${apiToken}` }
  })
}

const handle = async req => {
  const payload = await req.json()

  const zerotierRequest = await getZeroTierMember(payload.zerotier_address)
  const zerotierPayload = await zerotierRequest.json()

  await AGENT_ID_TO_IPV4.put(payload.holochain_agent_id.toLowerCase(), zerotierPayload.config.ipAssignments[0])
  return respond(200)
}

export { handle }
