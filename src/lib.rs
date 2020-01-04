#[cfg(test)]
extern crate reqwest;
extern crate futures;
extern crate tokio;
extern crate serde;
#[macro_use]
extern crate serde_derive;

// TODO: doku
// TODO: caching
// TODO: config
mod midata{

    #[derive(Deserialize, Debug)]
    struct Person {
        //TODO: add information about the user here
        id: String,
        href: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        nickname: Option<String>,
        company_name: Option<String>,
        company: bool,
        email: Option<String>,
        address: Option<String>,
        zip_code: Option<String>,
        town: Option<String>,
        country: Option<String>,
        picture: Option<String>,
    }

    #[derive(Deserialize, Debug)]
    struct Response{
        people: Option<Vec<Person>>,
        groups: Option<Vec<Group>>,
        linked: Option<GroupLinked>
    }

    #[derive(Deserialize, Debug)]
    struct Group {
        id : String,
        href : Option<String>,
        group_type : String,
        layer : Option<bool>,
        name : String,
        short_name : Option<String>,
        email : Option<String>,
        address : Option<String>,
        zip_code : Option<u16>,
        town : Option<String>,
        country :Option<String>,
        pbs_shortname : Option<String>,
        website : Option<String>,
        bank_account : Option<String>,
        description : Option<String>,
        // TODO: links and linked
        // TODO: personen
    }

    #[derive(Deserialize, Debug)]
    struct GroupLinked {
        groups: Option<Vec<Group>>
      }

    enum Request{
        Groups(u16),
        PeopleOfGroup(u16),
        People(u16, u16),
    }

    pub fn load_groups(ids: Vec<u16>){
        load(ids.into_iter().map(|id| Request::Groups(id)).collect())
    }
    pub fn load_people_of_group(ids: Vec<u16>){
        load(ids.into_iter().map(|id| Request::PeopleOfGroup(id)).collect())
    }
    pub fn load_people(ids: Vec<(u16, u16)>){
          load(ids.into_iter().map(|ids| Request::People(ids.0, ids.1)).collect())
    }

    #[tokio::main]
    async fn load(requests: Vec<Request>){
        let client = reqwest::Client::new();

        async fn _load_int(client: &reqwest::Client, request: Request) -> Response{
            let url = match request {
                Request::Groups(id) => {
                    format!("https://db.scout.ch/de/groups/{}", id)
                },
                Request::PeopleOfGroup(id) => {
                    format!("https://db.scout.ch/de/groups/{}/people", id)
                }
                Request::People(idg, idp) => {
                    format!("https://db.scout.ch/de/groups/{}/people/{}", idg, idp)
                }
            };

            println!("{}", url);
            let url = reqwest::Url::parse(&url).expect("Failed to parse url");
            // TODO: configure
            let body = client.get(url).header("X-Token","opa2xSCRzYU3eCeVz-gpHgjtLsXLq9vgPehzWzy3usBGW6fMZQ").header("Accept","application/json").send();
            let body = body.await.expect("Error loading body");

            body.json::<Response>().await.expect("Could not deserialize to json")
        }

        let requests: Vec<_> = requests.into_iter().map(|req| _load_int(&client, req)).collect();

        let responses: Vec<Response> = futures::future::join_all(requests).await;


        for res in responses{

        // TODO: check if logged in
        println!("#############################");
        // TODO: create type for body -> probably not because might change ....
        println!("body = {:?}", res);
        println!("#############################");
        // TODO: return sth
        }
        // TODO: handle errors
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn load_group() {
        crate::midata::load_groups(vec!(6497, 0));

        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn load_persons_of_group() {
        crate::midata::load_people_of_group(vec!(5763));

        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn load_persons() {
        crate::midata::load_people(vec!((5763,17773)));

        assert_eq!(2 + 2, 4);
    }

}
