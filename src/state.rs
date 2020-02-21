use couchbase::Cluster;

pub struct State {
    connstr: String,
    username: String,
    password: String,
    cluster: Cluster,
}

impl State {
    pub fn new(connstr: String, username: String, password: String) -> Self {
        let cluster = Cluster::connect(&connstr, &username, &password);
        Self {
            cluster,
            connstr,
            username,
            password,
        }
    }

    pub fn cluster(&self) -> &Cluster {
        &self.cluster
    }

    pub fn username(&self) -> &str {
        self.username.as_str()
    }

    pub fn password(&self) -> &str {
        self.password.as_str()
    }

    pub fn connstr(&self) -> &str {
        self.connstr.as_str()
    }
}
