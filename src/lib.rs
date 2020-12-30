extern crate futures;
#[cfg(test)]
extern crate reqwest;
extern crate serde;
extern crate tokio;
#[macro_use]
extern crate serde_derive;
extern crate cached;

/// Module for requesting and storing of information on Midata
pub mod midata {
    enum Token {
        XUserToken(String, String),
        XToken(String),
    }

    pub struct MidataConnection {
        token: Token,
    }

    /// Links of people to roles
    #[derive(Deserialize, Debug, Clone)]
    struct PersonLinks {
        #[serde(default)]
        /// primary group id of the person
        primary_group: String,
        /// Links to roles
        roles: Option<Vec<String>>,
    }

    /// Person in the midata database
    #[derive(Deserialize, Debug, Clone)]
    pub struct Person {
        pub id: String,
        /// url to page about that person
        pub href: Option<String>,
        pub first_name: Option<String>,
        pub last_name: Option<String>,
        /// scout's name
        pub nickname: Option<String>,
        pub company_name: Option<String>,
        pub company: bool,
        pub email: Option<String>,
        pub gender: Option<String>,
        pub address: Option<String>,
        pub zip_code: Option<String>,
        pub town: Option<String>,
        pub country: Option<String>,
        /// unique identifier of the household the person lives in
        pub household_key: Option<String>,
        /// url to the picture of the person
        pub picture: Option<String>,
        /// links to roles and primary group
        links: PersonLinks,

        /// Authentication token, only populated when logging in
        authentication_token: Option<String>,

        /// not mapped. facilitated access to roles
        #[serde(default)]
        pub roles: Vec<Role>,
        /// not mapped. when loaded from a group, not all fields are loaded/populated. Load remaining fields using load()
        #[serde(skip)]
        is_loaded_fully: bool,
        /// not mapped. when loaded from a group, the id of the group that loaded the person.
        #[serde(skip)]
        requested_by_group: u16,
        /// not mapped. utility to check if person has a leading function in any group.
        #[serde(skip)]
        pub is_leiter: bool,
        // NOTE: Update merge_persons if more fields are added
    }

    /// generic response from midata.
    #[derive(Deserialize, Debug, Clone)]
    struct Response {
        people: Option<Vec<Person>>,
        groups: Option<Vec<Group>>,
        linked: Option<Linked>,
    }

    /// Links for groups. Contains links to parent group and optionally the children groups
    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct GroupLinks {
        parent: String,
        layer_group: String,
        hierarchies: Option<Vec<String>>,
        children: Option<Vec<String>>,
    }

    /// Holds information about groups loaded from midata
    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Group {
        pub id: String,
        href: Option<String>,
        group_type: String,
        layer: Option<bool>,
        pub name: String,
        short_name: Option<String>,
        email: Option<String>,
        address: Option<String>,
        zip_code: Option<u16>,
        town: Option<String>,
        country: Option<String>,
        pbs_shortname: Option<String>,
        website: Option<String>,
        bank_account: Option<String>,
        description: Option<String>,
        pta: Option<bool>,
        vkp: Option<bool>,
        pbs_material_insurance: Option<bool>,
        links: Option<GroupLinks>,
        hierarchies: Option<Vec<Group>>,

        /// not mapped. Utility to hold the children groups of the group
        #[serde(skip)]
        pub chilrden: Vec<Group>,

        /// not mapped. Check if the group is fully loaded. Use the load() method if a non-loaded part of the group is needed.
        #[serde(skip)]
        is_loaded_fully: bool,

        /// not mapped. utility to hold the members of the group
        #[serde(skip)]
        people: Option<Vec<Person>>,
    }

    /// link to a group in a role
    #[derive(Deserialize, Debug, Clone)]
    pub struct RolesLinks {
        pub group: String,
        pub layer_group: String,
    }

    /// description of roles as loaded from midata
    #[derive(Deserialize, Debug, Clone)]
    pub struct Role {
        pub id: String,
        pub role_type: String,
        pub label: Option<String>,
        pub created_at: String,
        pub updated_at: String,
        pub deleted_at: Option<String>,
        pub links: Option<RolesLinks>,
    }

    /// generic container for links loaded from midata
    #[derive(Deserialize, Debug, Clone)]
    struct Linked {
        groups: Option<Vec<Group>>,
        roles: Option<Vec<Role>>,
    }

    /// generic structure to hold different request types to midata
    #[derive(Clone, PartialEq, Eq, Hash, Copy)]
    enum Request {
        Groups(u16),
        PeopleOfGroup(u16),
        People(u16, u32),
    }

    /// authenticate using an application token
    /// # NOTE:
    /// You will not get all the details about the groups/people when using this authentication method
    pub fn connection_with_application_token(token: String) -> MidataConnection {
        return MidataConnection {
            token: Token::XToken(token),
        };
    }

    /// authenticate using email and password
    pub fn connection_with_login(email: String, password: String) -> MidataConnection {
        let mut mc = MidataConnection {
            token: Token::XUserToken("".to_string(), "".to_string()),
        };

        mc.login(email, password);
        return mc;
    }

    /// authenticate using email and password
    pub fn connection_with_user_token(email: String, token: String) -> MidataConnection {
        let mc = MidataConnection {
            token: Token::XUserToken(email, token),
        };

        return mc;
    }

    impl Group {
        /// fully load the group if not already fully loaded.
        ///
        /// # Note
        /// This replaces the group object.
        /// Group-members are not altered.
        pub fn load(&mut self, connection: &MidataConnection) {
            if !self.is_loaded_fully {
                let mut group: Group = connection.load_group(self.id.parse().unwrap());
                group.people = self.people.to_owned();
                group.is_loaded_fully = true;
                *self = group;
            }
        }

        /// get members of a group. load the members if they are not loaded yet.
        ///
        /// # Note
        /// This does not fully load the persons. Use @ref get_persons_with_details
        /// if the details of the people are needed.
        pub fn get_persons<'a>(&'a mut self, connection: &MidataConnection) -> &'a Vec<Person> {
            if !self.people.is_some() {
                let id: u16 = self.id.parse().unwrap();
                self.people.replace(connection.load_people_of_group(id));
            }
            return self.people.as_ref().unwrap();
        }

        /// get members of a group. Load the members with full details if they are not loaded yet.
        /// Load details of person if not loaded yet.
        ///
        /// # Note
        /// Use @ref get_persons if details are not needed since it is much faster (especially for large
        /// groups.)
        pub fn get_persons_with_details<'a>(
            &'a mut self,
            connection: &MidataConnection,
        ) -> &'a Vec<Person> {
            let gid: u16 = self.id.parse().unwrap();
            let persons = self.get_persons(connection);
            let mut ids: Vec<(u16, u32)> = vec![];

            let mut result: Vec<Person> = vec![];
            for person in persons {
                if !person.is_loaded_fully {
                    ids.push((gid, person.id.parse().unwrap()));
                } else {
                    result.push(person.to_owned());
                }
            }
            result.append(&mut connection.load_people(ids));

            self.people.replace(result);

            return self.people.as_ref().unwrap();
        }
    }

    fn merge_option_if_needed<T>(option_a: &mut Option<T>, option_b: Option<T>) {
        if option_a.is_none() && option_b.is_some() {
            option_a.replace(option_b.unwrap());
        }
    }

    fn merge_option_vec_if_needed<T>(
        option_a: Option<Vec<T>>,
        option_b: Option<Vec<T>>,
    ) -> Option<Vec<T>> {
        if option_a.is_none() && option_b.is_some() {
            return option_b;
        } else if option_a.is_some() && option_b.is_some() {
            let mut vec1: Vec<T> = option_a.unwrap();
            vec1.append(&mut option_b.unwrap());
            return Some(vec1);
        }
        return option_a;
    }

    impl Person {
        /// fully load the person if not already fully loaded.
        ///
        /// # Note
        /// This replaces the Person object.
        pub fn load(&mut self, connection: &MidataConnection) {
            if !self.is_loaded_fully {
                *self = connection.load_person(self.requested_by_group, self.id.parse().unwrap());
                self.is_loaded_fully = true;
            }
        }

        /// Check if the person has a leader role in any group.
        ///
        /// # Note:
        /// This checks for the roles Biber, Wolf, Leitwolf, Pfadi, Leitpfadi, Pio
        pub fn is_tn(&self) -> bool {
            assert_ne!(self.roles.len(), 0);
            vec!["Biber", "Wolf", "Leitwolf", "Pfadi", "Leitpfadi", "Pio"]
                .iter()
                .any(|&tn_role| self.roles.iter().any(|r| r.role_type == tn_role))
        }

        fn merge_persons(&mut self, mut person: Person) {
            merge_option_if_needed(&mut self.email, person.email);
            merge_option_if_needed(&mut self.gender, person.gender);
            merge_option_if_needed(&mut self.address, person.address);
            merge_option_if_needed(&mut self.zip_code, person.zip_code);
            merge_option_if_needed(&mut self.town, person.town);
            merge_option_if_needed(&mut self.country, person.country);
            merge_option_if_needed(&mut self.household_key, person.household_key);
            merge_option_if_needed(&mut self.picture, person.picture);

            self.links.roles =
                merge_option_vec_if_needed(self.links.roles.clone(), person.links.roles);
            self.roles.append(&mut person.roles);
            self.is_loaded_fully = self.is_loaded_fully || person.is_loaded_fully;
            self.is_leiter = self.is_leiter || person.is_leiter;
        }
    }

    impl MidataConnection {
        /// Load a group
        ///
        /// # Arguments
        /// id: id of the group to load
        pub fn load_group(&self, id: u16) -> Group {
            return self.load_groups(vec![id]).pop().unwrap();
        }

        /// Load multiple groups
        ///
        /// # Arguments
        /// ids: ids of the group to load
        pub fn load_groups(&self, ids: Vec<u16>) -> Vec<Group> {
            let responses: Vec<Response> =
                self.load(ids.into_iter().map(|id| Request::Groups(id)).collect());
            let mut groups: Vec<Group> = vec![];

            // iterate over responses
            for r in responses {
                // check preconditions: groups present
                if let Some(response_groups) = r.groups {
                    // iterate over groups
                    for mut group in response_groups {
                        let mut group_hierarchies: Vec<Group> = vec![];
                        let mut group_children: Vec<Group> = vec![];

                        // check precondition: must have links, must have linked
                        if let Some(group_links) = &group.links {
                            if let Some(result_linked) = &r.linked {
                                // there must be groups linked
                                if let Some(linked_groups) = &result_linked.groups {
                                    // ready to roll

                                    // links must have hierarchy
                                    if let Some(group_links_hierarchy) = &group_links.hierarchies {
                                        // iterate and compare
                                        for hierarchy_group in group_links_hierarchy {
                                            for linked_group in linked_groups {
                                                if hierarchy_group == &linked_group.id {
                                                    let mut lg: Group = linked_group.to_owned();
                                                    lg.is_loaded_fully = false;
                                                    group_hierarchies.push(lg);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    // links must have children
                                    if let Some(group_links_children) = &group_links.children {
                                        // iterate and compare
                                        for child_group in group_links_children {
                                            for linked_group in linked_groups {
                                                if child_group == &linked_group.id {
                                                    let mut lg: Group = linked_group.to_owned();
                                                    lg.is_loaded_fully = false;
                                                    group_children.push(lg);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        group.chilrden = group_children;
                        group.hierarchies = Some(group_hierarchies);
                        group.is_loaded_fully = true;
                        groups.push(group);
                    }
                }
            }
            groups
        }

        /// Load details of persons
        ///
        /// # Arguments
        /// List of Persons for which we want to load the details.
        ///
        /// # Note
        /// This loads the details from the group where the person was initially loaded from as it is assumed that the person
        /// requesting the details has access to that group.
        pub fn load_details_of_people(&self, persons: Vec<Person>) -> Vec<Person> {
            let mut ids: Vec<(u16, u32)> = vec![];
            for person in persons {
                ids.push((person.requested_by_group, person.id.parse().unwrap()));
            }

            let mut loaded_persons = self.load_people(ids);
            loaded_persons.sort_by(|a, b| a.id.cmp(&b.id));

            let mut output: Vec<Person> = vec![];

            for person in loaded_persons {
                if output.len() > 0 && person.id == output.last().unwrap().id {
                    let last = output.last_mut().unwrap();
                    last.merge_persons(person);
                } else {
                    output.push(person);
                }
            }

            return output;
        }

        pub fn load_people_of_group(&self, id: u16) -> Vec<Person> {
            return self.load_people_of_groups(vec![id]);
        }

        pub fn load_people_of_groups(&self, ids: Vec<u16>) -> Vec<Person> {
            let responses: Vec<Response> = self.load(
                ids.into_iter()
                    .map(|id| Request::PeopleOfGroup(id))
                    .collect(),
            );
            let mut persons: Vec<Person> = vec![];
            for r in responses {
                if let Some(response_people) = r.people {
                    for mut person in response_people {
                        let mut person_roles: Vec<Role> = vec![];
                        if let Some(roles) = &person.links.roles {
                            if let Some(result_roles) = &r.linked {
                                if let Some(eff_roles) = &result_roles.roles {
                                    for role_string in roles {
                                        for role in eff_roles {
                                            if role_string == &role.id {
                                                person_roles.push(role.clone());
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        person.roles = person_roles;
                        person.is_leiter = !person.is_tn();
                        person.is_loaded_fully = false;
                        persons.push(person);
                    }
                }
            }
            persons
        }

        pub fn load_person(&self, gid: u16, id: u32) -> Person {
            return self.load_people(vec![(gid, id)]).pop().unwrap();
        }

        pub fn load_people(&self, ids: Vec<(u16, u32)>) -> Vec<Person> {
            let responses: Vec<Response> = self.load(
                ids.into_iter()
                    .map(|ids| Request::People(ids.0, ids.1))
                    .collect(),
            );
            let mut persons: Vec<Person> = vec![];
            for r in responses {
                if let Some(response_people) = r.people {
                    for mut person in response_people {
                        let mut person_roles: Vec<Role> = vec![];
                        if let Some(roles) = &person.links.roles {
                            if let Some(result_roles) = &r.linked {
                                if let Some(eff_roles) = &result_roles.roles {
                                    for role_string in roles {
                                        for role in eff_roles {
                                            if role_string == &role.id {
                                                person_roles.push(role.clone());
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        person.roles = person_roles;
                        person.is_leiter = !person.is_tn();
                        person.is_loaded_fully = true;
                        persons.push(person);
                    }
                }
            }
            persons
        }

        fn login(&mut self, email: String, password: String) {
            #[tokio::main]
            #[cached::proc_macro::cached(
                size = 1,
                convert = "{ format!(\"{}\", email) }",
                key = "String"
            )]
            async fn get_token(email: &str, password: &str) -> String {
                let url = reqwest::Url::parse("https://db.scout.ch/users/sign_in.json")
                    .expect("parse error");
                let client = reqwest::Client::new();
                let params = [("person[email]", &email), ("person[password]", &password)];
                let response = client
                    .post(url)
                    .header("Accept", "application/json")
                    .form(&params)
                    .send()
                    .await
                    .expect("Failed to authenticate with midata.");
                let response = response
                    .json::<Response>()
                    .await
                    .expect("Could not deserialize to json");
                return response
                    .people
                    .unwrap()
                    .pop()
                    .unwrap()
                    .authentication_token
                    .unwrap();
            }
            let token = get_token(&email, &password);
            self.token = Token::XUserToken(email, token);
        }

        #[tokio::main]
        async fn load(&self, requests: Vec<Request>) -> Vec<Response> {
            let client = reqwest::Client::new();

            #[cached::proc_macro::cached(size = 1000, convert = "{request}", key = "Request")]
            async fn _load_int(
                client: &reqwest::Client,
                request: Request,
                credentials: &MidataConnection,
            ) -> Response {
                let url = match request {
                    Request::Groups(id) => {
                        format!("https://db.scout.ch/de/groups/{}", id)
                    }
                    Request::PeopleOfGroup(id) => {
                        format!("https://db.scout.ch/de/groups/{}/people", id)
                    }
                    Request::People(idg, idp) => {
                        format!("https://db.scout.ch/de/groups/{}/people/{}", idg, idp)
                    }
                };
                println!("{}", url);
                let url = reqwest::Url::parse(&url).expect("Failed to parse url");

                let mut headers = reqwest::header::HeaderMap::new();

                match &credentials.token {
                    Token::XToken(token) => {
                        headers.insert(
                            "X-Token",
                            reqwest::header::HeaderValue::from_str(&token).unwrap(),
                        );
                    }
                    Token::XUserToken(user, token) => {
                        headers.insert(
                            "X-User-Token",
                            reqwest::header::HeaderValue::from_str(&token).unwrap(),
                        );
                        headers.insert(
                            "X-User-Email",
                            reqwest::header::HeaderValue::from_str(&user).unwrap(),
                        );
                    }
                }
                headers.insert(
                    "Accept",
                    reqwest::header::HeaderValue::from_static(&"application/json"),
                );

                let body = client.get(url).headers(headers).send();

                let body = body.await.expect("Error loading body");
                let mut response = body
                    .json::<Response>()
                    .await
                    .expect("Could not deserialize to json");
                if let Request::PeopleOfGroup(id) = request {
                    if let Some(people) = &mut response.people {
                        for person in people {
                            person.requested_by_group = id;
                        }
                    }
                }
                response
            }

            let mut remining_requests = requests.as_slice();
            let mut responses: Vec<Response> = vec![];
            while remining_requests.len() > 0 {
                let index = std::cmp::min(100, remining_requests.len());
                let split_req = remining_requests.split_at(index);
                remining_requests = split_req.1;

                let t_requests: Vec<_> = split_req
                    .0
                    .into_iter()
                    .map(|req| _load_int(&client, req.to_owned(), self))
                    .collect();
                let mut t_responses: Vec<Response> = futures::future::join_all(t_requests).await;
                responses.append(&mut t_responses);
            }

            responses
        }
    }
}

#[cfg(test)]
mod tests {
    fn login() -> crate::midata::MidataConnection {
        //crate::midata::connection_with_login("XXX".to_string(), "XXX".to_string())
        crate::midata::connection_with_application_token("XXX".to_string())
    }

    #[test]
    fn load_group() {
        let mc = login();
        let res = mc.load_groups(vec![6497, 0]);
        for r in res {
            println!("{:?}", r);
        }
    }

    #[test]
    fn load_persons_of_group() {
        let mc = login();
        let res = mc.load_people_of_groups(vec![5763]);
        for r in res {
            println!("{:?}", r);
        }
    }

    #[test]
    fn load_persons() {
        let mc = login();
        let res = mc.load_people(vec![(5763, 17773)]);
        for r in res {
            println!("{:?}", r);
        }
    }

    #[test]
    fn load_steps() {
        let mc = login();
        let mut group = mc.load_group(6497);
        group.load(&mc);
        let people = group.get_persons(&mc);
        let mut person = people[0].clone();
        person.load(&mc);
    }

    #[test]
    fn person_has_household_key() {
        let mc = login();
        let res = mc.load_person(6497, 3967);
        assert!(res.household_key.is_some());
    }

    #[test]
    fn person_has_no_household_key() {
        let mc = login();
        let res = mc.load_person(6497, 57306);
        assert!(res.household_key.is_none());
    }
}
