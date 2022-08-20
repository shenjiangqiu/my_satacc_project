#[allow(unused_macros)]
macro_rules! impl_for_tuples {
    ($id1:ident;$id2:ident;($($i:ident),+$(,)?)$(,)?) => {
        impl_for_one_tuple!($id1;$id2;($($i,)*));

    };
    ($id1:ident;$id2:ident;($($i:ident),+$(,)?),$(($($else:ident),+$(,)?)),*$(,)?) => {
        impl_for_one_tuple!($id1;$id2;($($i,)*));
        impl_for_tuples!($id1;$id2;$(($($else,)*),)*);
    };
}
#[allow(unused_macros)]
macro_rules! impl_for_one_tuple {
    ($id1:ident;$id2:ident;($($i:ident),+$(,)?)) => {
        impl<$($i:$id1,)*> $id1 for ($($i,)*) {
            fn $id2(&self) -> bool {
                let mut temp=false;
                let ( $(ref $i,)*) = *self;
                $(
                    temp = $i.$id2() || temp;
                )*
                temp
            }
        }
    };
}

#[allow(unused_macros)]
macro_rules! impl_for_tuples_with_type {
    ($id1:ident;$id2:ident;$id3:ident;($($i:ident),+$(,)?)$(,)?) => {
        impl_for_one_tuple_with_type!($id1;$id2;$id3;($($i,)*));

    };
    ($id1:ident;$id2:ident;$id3:ident;($($i:ident),+$(,)?),$(($($else:ident),+$(,)?)),*$(,)?) => {
        impl_for_one_tuple_with_type!($id1;$id2;$id3;($($i,)*));
        impl_for_tuples_with_type!($id1;$id2;$id3;$(($($else,)*),)*);
    };
}
#[allow(unused_macros)]
macro_rules! impl_for_one_tuple_with_type {
    ($id1:ident;$id2:ident;$id3:ident;($($i:ident),+$(,)?)) => {
        impl<TypeOfTrait,$($i:$id1<$id3=TypeOfTrait>,)*> $id1 for ($($i,)*) {
            type $id3 = TypeOfTrait;
            fn $id2(&mut self,status:&mut Self::$id3,cycle:usize) -> (bool,bool) {
                let mut busy=false;
                let mut updated=false;
                let ( $(ref mut $i,)*) = *self;
                $(
                    let (tbusy,tupdated) = $i.$id2(status,cycle);
                    busy = busy || tbusy;
                    updated = updated || tupdated;
                )*
                (busy,updated)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    trait Foo {
        fn foo(&self) -> bool;
    }
    impl Foo for i32 {
        fn foo(&self) -> bool {
            true
        }
    }
    impl Foo for f32 {
        fn foo(&self) -> bool {
            false
        }
    }
    trait Bar {
        type Doo;
        fn bar(&mut self, status: &mut Self::Doo, cycle: usize) -> (bool, bool);
    }
    impl Bar for i32 {
        type Doo = ();
        fn bar(&mut self, _status: &mut Self::Doo, _cycle: usize) -> (bool, bool) {
            (true, true)
        }
    }
    impl Bar for f32 {
        type Doo = ();
        fn bar(&mut self, _status: &mut Self::Doo, _cycle: usize) -> (bool, bool) {
            (false, false)
        }
    }
    impl<T> Bar for &mut T where T: Bar {
        type Doo = T::Doo;
        fn bar(&mut self, status: &mut Self::Doo, cycle: usize) -> (bool, bool) {
            (*self).bar(status, cycle)
        }
    }
    #[test]
    #[allow(non_camel_case_types)]
    fn test_impl_tuple() {
        impl_for_tuples!(Foo;foo;(a),
                                (a,b),
                                (a,b,c),
                                (a,b,c,d),  
                                (a,b,c,d,e),    
                                (a,b,c,d,e,f),  
                                (a,b,c,d,e,f,g),
                                (a,b,c,d,e,f,g,h),);
        let result = (1, 2, 3).foo();
        assert_eq!(result, true);
        let result = (1, 2, 3.0, 4).foo();
        assert_eq!(result, true);
        let result = (1., 2., 3.0, 4.0).foo();
        assert_eq!(result, false);
    }
    // #[test]
    // #[allow(non_camel_case_types)]
    // fn test_impl_tuple_with_type() {
    //     impl_for_tuples_with_type!(Bar;bar;Doo;(a),
    //                             (a,b),
    //                             (a,b,c),
    //                             (a,b,c,d),
    //                             (a,b,c,d,e),
    //                             (a,b,c,d,e,f),
    //                             (a,b,c,d,e,f,g),
    //                             (a,b,c,d,e,f,g,h),);
    //     let mut status = ();

    //     let result = (1, 2, 3).bar(&mut status, 0);
    //     assert_eq!(result, (true, true));
    //     let result = (1, 2, 3.0, 4).bar(&mut status, 0);
    //     assert_eq!(result, (false, false));
    // }

    #[test]
    #[allow(non_camel_case_types)]
    fn test_ref() {
        impl_for_tuples_with_type!(Bar;bar;Doo;(a),
                                (a,b),
                                (a,b,c),
                                (a,b,c,d),  
                                (a,b,c,d,e),    
                                (a,b,c,d,e,f),  
                                (a,b,c,d,e,f,g),
                                (a,b,c,d,e,f,g,h),);
        let mut status = ();
        let result = (&mut 1, &mut 2, &mut 3).bar(&mut status, 0);
        assert_eq!(result, (true, true));
        let result = (&mut 1, &mut 2, &mut 3.0, &mut 4).bar(&mut status, 0);
        assert_eq!(result, (true,true));
        let result = (&mut 1., &mut 2., &mut 3., &mut 4.0).bar(&mut status, 0);
        assert_eq!(result, (false,false));
        let result = (1,2,3,4).bar(&mut status, 0);
        assert_eq!(result, (true,true));
    }
}
