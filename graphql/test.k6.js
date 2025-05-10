import http from 'k6/http';

const accessToken = 'YOUR_GITHUB_ACCESS_TOKEN';
    
const query = `
 {
  human(id: "1234") {
    name
    appearsIn
    homePlanet
  }
}`;

const headers = {
  'Authorization': `Bearer ${accessToken}`,
  'Content-Type': 'application/json',
};


export const options = {
    stages: [
      { duration: '1m30s', target: 10 },
      { duration: '1m30s', target: 20 },
      { duration: '1m30s', target: 30 },
      { duration: '1m30s', target: 40 },
      { duration: '1m30s', target: 50 },
      { duration: '1m30s', target: 60 },
      { duration: '1m30s', target: 70 },
      { duration: '1m30s', target: 80 },
      { duration: '1m30s', target: 90 },
      { duration: '1m30s', target: 100 },
  
    ],
  };
  

export default function () {
    
    const res = http.post('http://localhost:8080/graphql', JSON.stringify({ query: query }), {
      headers: headers,
    });
    
  console.log(res.body);
}



