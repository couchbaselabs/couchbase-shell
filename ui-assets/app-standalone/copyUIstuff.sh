#!/bin/sh
#
# the standalone app relies upon a bunch of components from ns-server. 
#
# this script copies them over.
#
cp -r ../../../ns_server/priv/public/ui/app/components .
cp -r ../../../ns_server/priv/public/ui/app/constants .
cp -r ../../../ns_server/priv/public/ui/app/css .
cp -r ../../../ns_server/priv/public/ui/app/mn_admin .
cp -r ../../../ns_server/priv/public/ui/app/mn_auth .
cp -r ../../../ns_server/priv/public/ui/app/mn_wizard .

