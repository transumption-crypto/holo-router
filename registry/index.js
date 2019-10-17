const dnsPacket = require('dns-packet')

const dnsAnswer = async question => {
  if (question.class == 'IN' && question.type == 'A') {
    const [hcid, domain, tld] = question.name.split('.').slice(-3)

    if (domain == 'holohost' && tld == 'net') {
      const ipv4 = await HCID_TO_IPV4.get(hcid)

      if (ipv4 != null) {
        return [{
          class: 'IN',
          data: ipv4,
          name: question.name,
          ttl: 300, // 5 minutes
          type: 'A'
        }]
      }
    }
  }
}

const dnsQuery = async req => {
  const reqBuffer = Buffer.from(await req.arrayBuffer())
  const reqPacket = dnsPacket.decode(reqBuffer)

  if (reqPacket.questions.length != 1) {
    return new Response(null, { status: 400 })
  }

  const resPacket = {
    type: 'response',
    questions: reqPacket.questions,
    answers: await dnsAnswer(reqPacket.questions[0]) || []
  }

  return new Response(dnsPacket.encode(resPacket), {
    headers: { 'content-type': 'application/dns-message' }
  })
}

const handleRequest = dnsQuery

addEventListener('fetch', event => {
  event.respondWith(handleRequest(event.request))
})
