use crate::{error::Error, name::Name, util::OidSet};
use libgssapi_sys::{
    gss_OID_set, gss_acquire_cred, gss_cred_id_struct, gss_cred_id_t, gss_cred_usage_t,
    gss_mech_krb5, gss_name_struct, gss_release_cred, OM_uint32, GSS_C_ACCEPT,
    GSS_C_BOTH, GSS_C_INITIATE, GSS_S_COMPLETE, _GSS_C_INDEFINITE,
};
use std::{ops::Deref, ptr, fmt, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub enum CredUsage {
    Accept,
    Initiate,
    Both,
}

struct CredInner(gss_cred_id_t);

impl Drop for CredInner {
    fn drop(&mut self) {
        let mut minor = GSS_S_COMPLETE;
        let _major = unsafe {
            gss_release_cred(
                &mut minor as *mut OM_uint32,
                &mut self.0 as *mut gss_cred_id_t,
            )
        };
        // CR estokes: log errors? panic?
    }
}

#[derive(Clone)]
pub struct Cred(Arc<CredInner>);

impl fmt::Debug for Cred {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "<gss credential>")
    }
}

// CR estokes: improve this, it can be much much better
impl Deref for Cred {
    type Target = gss_cred_id_t;

    fn deref(&self) -> &Self::Target {
        &(self.0).0
    }
}

impl Cred {
    pub fn acquire(
        name: Option<&Name>,
        time_req: Option<u32>,
        usage: CredUsage,
    ) -> Result<Cred, Error> {
        let name = name
            .map(|n| **n)
            .unwrap_or(ptr::null_mut::<gss_name_struct>());
        let time_req = time_req.unwrap_or(_GSS_C_INDEFINITE);
        let mut desired_mechs = {
            let mut s = OidSet::new()?;
            unsafe { s.add(gss_mech_krb5)? };
            s
        };
        let usage = match usage {
            CredUsage::Both => GSS_C_BOTH,
            CredUsage::Initiate => GSS_C_INITIATE,
            CredUsage::Accept => GSS_C_ACCEPT,
        };
        let mut minor = GSS_S_COMPLETE;
        let mut cred = ptr::null_mut::<gss_cred_id_struct>();
        let major = unsafe {
            gss_acquire_cred(
                &mut minor as *mut OM_uint32,
                name,
                time_req,
                desired_mechs.as_ptr(),
                usage as gss_cred_usage_t,
                &mut cred as *mut gss_cred_id_t,
                ptr::null_mut::<gss_OID_set>(),
                ptr::null_mut::<OM_uint32>(),
            )
        };
        if major == GSS_S_COMPLETE {
            Ok(Cred(Arc::new(CredInner(cred))))
        } else {
            Err(Error { major, minor })
        }
    }

    pub(crate) fn from_raw(cred: gss_cred_id_t) -> Cred {
        Cred(Arc::new(CredInner(cred)))
    }
}