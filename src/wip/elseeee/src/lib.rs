#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(impl_trait_in_assoc_type)]
#![allow(unused_doc_comments)]

#[cfg(test)]
mod tests {
	//use super::*;
	use anyhow::Result as Rslt;

	#[test]
	fn moku() -> Rslt<(),> {
		let moku = String::from_utf8(vec![
			227, 130, 130, 227, 129, 143, 227, 130, 130, 227, 129, 143, 227, 129, 151, 227, 129,
			190, 227, 129, 153,
		],)?;
		assert_eq!("もくもくします", moku);

		let my_fav_lang = unsafe {
			String::from_utf8_unchecked(vec![
				82, 117, 115, 116, 227, 129, 168, 108, 117, 97, 227, 129, 140, 229, 165, 189, 227,
				129, 141, 227, 129, 167, 227, 130, 136, 227, 129, 143, 232, 167, 166, 227, 129,
				163, 227, 129, 166, 227, 129, 132, 227, 129, 190, 227, 129, 153, 32, 229, 184, 131,
				230, 149, 153, 227, 129, 151, 227, 129, 159, 227, 129, 132,
			],)
		};
		assert_eq!("Rustとluaが好きでよく触っています 布教したい", my_fav_lang);
		Ok((),)
	}

	#[test]
	fn sqlite() -> Rslt<(),> {
		let connect = rusqlite::Connection::open_in_memory()?;
		connect.execute_batch(
			"create table if not exists articles (
				id integer primary key,
				date text not null
		);",
		)?;

		let should_one = connect.execute(
			"insert into articles (date) values (?1)",
			&[&chrono::Local::now().timestamp(),],
		)?;

		assert_eq!(1, should_one);

		Ok((),)
	}

	#[test]
	fn understand_closure() {
		let x = 666;

		/// ---
		/// closure **syntax**
		(0..10).for_each(|i| {
			let cls = |arg| i + arg;

			assert_eq!(i + 666, cls(x));
		},);

		/// ---
		/// reproducing closure which only implements
		/// **FnOnce**
		#[derive(Clone,)]
		struct ClosureFnOnce {
			i: isize,
		}

		impl FnOnce<(isize,),> for ClosureFnOnce {
			type Output = isize;

			extern "rust-call" fn call_once(self, (args,): (isize,),) -> Self::Output {
				self.i + args
			}
		}

		let cls_fn_once_outer = ClosureFnOnce { i: x, };
		(0..10).for_each(|i| {
			assert_eq!(666, cls_fn_once_outer.i,);
			assert_eq!(666 + i, cls_fn_once_outer.clone()(i));
			// this will cause compile error because `cls_fn_once_outer` will be moved in a loop
			// `assert_eq!(1332, cls_fn_once_outer.call_once(x));`

			let cls_fn_once = ClosureFnOnce { i, };
			assert_eq!(i, cls_fn_once.i);
			assert_eq!(666 + i, cls_fn_once.clone()(x));

			// this will cause failing next `call_once` call because `ClosureFnOnce` will be moved
			// on this call. this behavior comes from `FnOnce` enforces move
			// `assert_eq!(666 + i, cls_fn_once.clone()(x));`

			assert_eq!(666 + i, cls_fn_once.clone().call_once((x,)));
			assert_eq!(i, cls_fn_once.i);
			assert_eq!(666 + i, cls_fn_once.call_once((x,)));

			// uncommenting code below cause compile error due to `cls_fn_once` is moved
			// `assert_eq!(i, cls_fn_once.i);`
		},);

		/// ---
		/// reproducing closure which implements
		/// **FnMut (that implies implementation of FnOnce exists)**
		#[derive(Clone,)]
		struct ClosureFnMut {
			i: isize,
		}

		impl FnMut<(isize,),> for ClosureFnMut {
			/// `type Output = ...` is not required because `FnOnce` is super trait of `FnMut`

			extern "rust-call" fn call_mut(&mut self, (args,): (isize,),) -> Self::Output {
				self.i += args;
				self.i
			}
		}

		/// this impl is necessary to `impl FnMut for ClosureFnMut` because FnMut takes FnOnce as a
		/// super trait
		impl FnOnce<(isize,),> for ClosureFnMut {
			type Output = isize;

			extern "rust-call" fn call_once(self, (arg,): (isize,),) -> Self::Output {
				self.i + arg
			}
		}

		impl FnMut<(),> for ClosureFnMut {
			extern "rust-call" fn call_mut(&mut self, _: (),) -> Self::Output {
				self.i *= 2;
				TryInto::<Self::Output,>::try_into(self.i,).unwrap()
			}
		}

		impl FnOnce<(),> for ClosureFnMut {
			type Output = u32;

			extern "rust-call" fn call_once(self, _: (),) -> Self::Output {
				TryInto::<Self::Output,>::try_into(self.i * 10,).unwrap()
			}
		}

		// impl ClosureFnMut {
		// 	fn static_ref<'a,>(i: isize,) -> &'a mut isize {
		// 		macro_rules! exp_as_tt {
		// 			($i:expr) => {
		// 				stringify!($i)
		// 			};
		// 		}
		//
		// 		let stringified = exp_as_tt!(i);
		// 		stringified.parse().expect("failed to parse &str as isize",)
		// 	}
		// }

		let mut store = 0;
		let mut cls_fn_mut_outer = ClosureFnMut { i: store, };
		(0..10).for_each(|i| {
			assert_eq!(store + i, cls_fn_mut_outer(i));
			assert_eq!(store + i * 2, cls_fn_mut_outer.clone().call_once((i,)));
			assert_eq!((store + i) * 2, cls_fn_mut_outer().try_into().unwrap());
			store = cls_fn_mut_outer.clone().call_once((0,),);

			/// `mut` keyword is required because calling `call_mut()` means `&mut self` is assured
			let mut cls_fn_mut = ClosureFnMut { i, };
			assert_eq!(i + i, cls_fn_mut(i));
			assert_eq!(i * 3, cls_fn_mut.clone().call_once((i,)));
			assert_eq!(i * 3, cls_fn_mut.call_mut((i,)));
			assert_eq!(i * 4, cls_fn_mut.clone()(i));
			assert_eq!(i * 4, cls_fn_mut(i));
			assert_eq!(cls_fn_mut.i + i, cls_fn_mut(i));
			assert_eq!(cls_fn_mut(i), cls_fn_mut.i);

			assert_eq!(i * 6 * 2, cls_fn_mut().try_into().unwrap());
			assert_eq!(cls_fn_mut.i * 2, cls_fn_mut.clone()().try_into().unwrap());
			assert_eq!(cls_fn_mut.i * 2, cls_fn_mut.clone().call_mut(()).try_into().unwrap());
			assert_eq!(i * 6 * 2 * 10, cls_fn_mut.clone().call_once(()).try_into().unwrap());

			let mut cls_fn_mut2 = cls_fn_mut.clone();
			cls_fn_mut2.i = 666;
			assert_eq!(1332, cls_fn_mut2.call_mut(()));
			assert_eq!(13320, cls_fn_mut2.call_once(()));

			let cls_fn_wont_mut = ClosureFnMut { i: cls_fn_mut(i,), };
			assert_eq!(i * (6 * 2 + 1), cls_fn_wont_mut.i);
			assert_eq!(i * (6 * 2 + 1 + 1), cls_fn_wont_mut.clone().call_once((i,)));
			assert_eq!(i * (6 * 2 + 1), cls_fn_wont_mut.i);
		},);

		/// ---
		/// reproducing closure which implements **Fn**
		let mut s = "closure opens up further possibilities to a program".to_string();
		struct ClosureFn<'a,> {
			i:      isize,
			s:      &'a mut String,
			orig_s: String,
		}

		impl<'a,> Fn<(),> for ClosureFn<'a,> {
			extern "rust-call" fn call(&self, _: (),) -> Self::Output {
				let mut s: Vec<_,> = self.s.split_whitespace().collect();
				s.sort();
				s.join(" ",)
			}
		}

		impl<'a,> FnMut<(),> for ClosureFn<'a,> {
			extern "rust-call" fn call_mut(&mut self, _: (),) -> Self::Output {
				(0..self.i).for_each(|_| {
					self.s.push(' ',);
					self.s.push_str(self.orig_s.clone().as_str(),);
				},);
				self.s.clone()
			}
		}

		impl<'a,> FnOnce<(),> for ClosureFn<'a,> {
			type Output = String;

			extern "rust-call" fn call_once(self, _: (),) -> Self::Output {
				format!("{}{}", self.i, self.s)
			}
		}

		let orig_s = s.clone();
		let repeat = |i| {
			//let mut s_rep = "".to_string();
			(0..i).map(|_| orig_s.clone(),).collect::<Vec<String,>>().join(" ",)
		};

		(0..10).for_each(|i| {
			let mut closure_fn = ClosureFn { i, s: &mut s.clone(), orig_s: orig_s.clone(), };
			assert_eq!("a closure further opens possibilities program to up", closure_fn());
			assert_eq!(repeat(i + 1), closure_fn.call_mut(()));
			assert_eq!(format!("{i}{}", repeat(i + 1)), closure_fn.call_once(()));
		},);
		let mut closure_fn_outer = ClosureFn { i: 2, s: &mut s, orig_s: orig_s.clone(), };
		let mutated_s = closure_fn_outer.call_mut((),);
		let rep_3_s = repeat(3,);
		assert_eq!(rep_3_s, mutated_s);
		assert_eq!(format!("2{rep_3_s}"), closure_fn_outer.call_once(()));

		// we do not need manually drop because previous call of `call_once` moves
		// `closure_fn_outer`
		// drop(closure_fn_outer,);
		assert_eq!(rep_3_s, s);
	}
}
