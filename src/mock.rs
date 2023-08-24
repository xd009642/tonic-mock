

pub struct MockMethod {
    
}


pub struct MockService {

}

// One thought of an approach is use a proc macro to generate a {NAME}MockService where we have
// default implementations for all methods and then we add in MockMethod to overwrite the default
// one. Then {Name}MockService we can add to a grpc service using add_service
