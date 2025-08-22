# This files contains a collection of scripts useful for CI tasks, like cloning buckets, scopes or collections

# Clones an entire bucket, scopes and collections
#
# All parameters can be null, Env Variables can be used. If
# the parameter is null and no env variables are set, param
# will default to current cb-env.
#
# SRC_CLUSTER: Source cluster identifier
# SRC_BUCKET: Source Bucket
# SRC_SCOPE: Source Scope
# SRC_COLLECTION: Source Collection
# DEST_CLUSTER: Destination cluster identifier
# DEST_BUCKET: Destination Bucket
# DEST_SCOPE: Destination Scope
# DEST_COLLECTION: Destination Collection
def bucket-clone [
    bucket?: string, # Name of the source bucket
    destbucket?: string, # Name of the destination bucket
    --source: string, # Identifier of the source Cluster
    --destination: string  # Identifier of the destination Cluster
    --with-indexes # copy all indexes in the bucket
] {
    run_with_default { |p|
        copy-bucket-definition $p.src_bucket $p.dest_bucket --source $p.src --destination $p.dest
        let scopes = scopes --clusters $p.src --bucket $p.src_bucket
        for scope in $scopes {
            scope-clone $p.src_bucket $scope.scope $p.dest_bucket $scope.scope --source $p.src --destination $p.dest
        }
        if ( $with_indexes ) {
            let indexes = query indexes --definitions --disable-context --clusters $p.src | where bucket == $p.src_bucket
            for index in $indexes {
                print $"Recreating index ($index.name) on cluster ($p.dest) with: "
                print $index.definition
                query $index.definition --disable-context --clusters $p.dest
            }
        }
    } --bucket $bucket --destbucket $destbucket --source $source --destination $destination
}

# Clones an entire Scope and its collections
#
# All parameters can be null, Env Variables can be used. If
# the parameter is null and no env variables are set, param
# will default to current cb-env.
#
# SRC_CLUSTER: Source cluster identifier
# SRC_BUCKET: Source Bucket
# SRC_SCOPE: Source Scope
# SRC_COLLECTION: Source Collection
# DEST_CLUSTER: Destination cluster identifier
# DEST_BUCKET: Destination Bucket
# DEST_SCOPE: Destination Scope
# DEST_COLLECTION: Destination Collection
def scope-clone [
    bucket?: string, # Name of the source bucket
    scope?: string, # Name of the source scope
    destbucket?: string, # Name of the destination bucket
    destscope?: string, # Name of the destination scope
    --source: string, # Identifier of the source Cluster
    --destination: string # Identifier of the destination Cluster
] {
    run_with_default { |p|
        if ( scopes --clusters $p.dest --bucket $p.dest_bucket | where scope == $p.dest_scope | is-empty ) {
            print $"Create scope ($p.dest)_($p.dest_bucket)_($p.dest_scope)"
            scopes create --clusters $p.dest --bucket $p.dest_bucket $p.dest_scope
        }
        let collections = collections --clusters $p.src --bucket $p.src_bucket --scope $p.src_scope
        for col in $collections {
            collection-clone $p.src_bucket $p.src_scope $col.collection $p.dest_bucket $p.dest_scope $col.collection  --source $p.src --destination $p.dest
        }
    } --bucket $bucket --destbucket $destbucket  --scope $scope --destscope $destscope --source $source --destination $destination
}

# Clones a collection
#
# All parameters can be null, Env Variables can be used. If
# the parameter is null and no env variables are set, param
# will default to current cb-env.
#
# SRC_CLUSTER: Source cluster identifier
# SRC_BUCKET: Source Bucket
# SRC_SCOPE: Source Scope
# SRC_COLLECTION: Source Collection
# DEST_CLUSTER: Destination cluster identifier
# DEST_BUCKET: Destination Bucket
# DEST_SCOPE: Destination Scope
# DEST_COLLECTION: Destination Collection
def collection-clone [
    bucket?: string, # Name of the source bucket
    scope?: string, # Name of the source scope
    collection?: string, # Name of the source collection
    destbucket?: string, # Name of the destination bucket
    destscope?: string, # Name of the destination scope
    destcollection?: string, # Name of the destination collection
    --source: string, # Identifier of the source Cluster
    --destination: string  # Identifier of the destination Cluster
] {
    run_with_default { |p|
        if ( collections --clusters $p.dest --bucket $p.dest_bucket --scope $p.dest_scope | where collection == $p.dest_collection | is-empty ) {
            print $"Create collection ($p.dest)_($p.dest_bucket)_($p.dest_scope)_($p.dest_collection)"
            collections create --clusters $p.dest --bucket $p.dest_bucket --scope  $p.dest_scope $p.dest_collection
        }
        let filename = $"temp_($p.src_bucket)_($p.src_scope)_($p.src_collection).json"
        let query = "SELECT meta().id as meta_id, meta().expiration as expiration, c.* FROM `" + $p.src_bucket + "`." + $p.src_scope + "." + $p.src_collection + " c"
        query --disable-context --clusters $p.src $query | save -f $filename
        print $"Import collection content from ($p.src)_($p.src_bucket)_($p.src_scope)_($p.src_collection) to ($p.dest)_($p.dest_bucket)_($p.dest_scope)_($p.dest_collection)"
        print (doc import --bucket $p.dest_bucket --scope $p.dest_scope --collection $p.dest_collection --clusters $p.dest --id-column meta_id $filename)
        
    } --bucket $bucket --destbucket $destbucket  --scope $scope --destscope $destscope   --collection $collection --destcollection $destcollection  --source $source --destination $destination
}

# Create another bucket based on source bucket configuration.
#
# All parameters can be null, Env Variables can be used. If
# the parameter is null and no env variables are set, param
# will default to current cb-env.
#
# SRC_CLUSTER: Source cluster identifier
# SRC_BUCKET: Source Bucket
# DEST_CLUSTER: Destination cluster identifier
# DEST_BUCKET: Destination Bucket
def copy-bucket-definition [
    bucket?: string, # Name of the source bucket
    destbucket?: string, # Name of the destination bucket
    --source: string, # Identifier of the source Cluster
    --destination: string  # Identifier of the destination Cluster
] {
    run_with_default { |p|
        let clonable = buckets get --clusters $p.src $p.src_bucket | get 0 
        print $"Create Bucket ($p.dest)_($p.dest_bucket) with ($clonable.ram_quota) quota, type ($clonable.type), ($clonable.replicas) replicas, ($clonable.min_durability_level) durability, ($clonable.max_expiry) expiry"
        if ( $clonable.flush_enabled) {
            $clonable | buckets create $p.dest_bucket ( $in.ram_quota / 1MB  | into int ) --clusters $p.dest  --type $in.type --replicas $in.replicas --durability $in.min_durability_level --expiry $in.max_expiry --flush
        } else {
            $clonable | buckets create $p.dest_bucket ( $in.ram_quota / 1MB | into int ) --clusters $p.dest  --type $in.type --replicas $in.replicas --durability $in.min_durability_level --expiry $in.max_expiry
        }
    } --bucket $bucket --destbucket $destbucket --source $source --destination $destination
}


# Run the given closure with an object containing all needed
# parameters.
#
# Null parameters are replaced by env variable if given. It
# defaults to current cb-env if nothing is available.
def run_with_default [
    operation: closure,
    --bucket: string,
    --scope: string,
    --collection: string,
    --destbucket: string,
    --destscope: string,
    --destcollection: string,
    --source: string,
    --destination: string
 ] {
    let src_bucket = if ($bucket != null) {
        $bucket
    } else if ( $env.SRC_BUCKET? != null ) {
        $env.SRC_BUCKET
    } else {
        cb-env | get bucket
    }
    let src_scope = if ($scope != null) {
        $scope
    } else if ( $env.SRC_SCOPE? != null ) {
        $env.SRC_SCOPE
    } else {
        cb-env | get scope
    }
    let src_collection = if ($collection != null) {
        $collection
    } else if ( $env.SRC_COLLECTION? != null ) {
        $env.SRC_COLLECTION
    } else {
        cb-env | get collection
    }

    let dest_bucket = if ($destbucket != null) {
        $destbucket
    } else if ( $env.DEST_BUCKET? != null ) {
        $env.DEST_BUCKET
    } else {
        cb-env | get bucket
    }
    let dest_scope = if ($destscope != null) {
        $destscope
    } else if ( $env.DEST_SCOPE? != null ) {
        $env.DEST_SCOPE
    } else {
        cb-env | get scope
    }
    let dest_collection = if ($destcollection != null) {
        $destcollection
    } else if ( $env.DEST_COLLECTION? != null ) {
        $env.DEST_COLLECTION
    } else {
        cb-env | get collection
    }

    let src_cluster = if ($source != null) {
        $source
    } else if ( $env.SRC_CLUSTER? != null ) {
        $env.SRC_CLUSTER
    } else {
        cb-env | get cluster
    }
    let dest_cluster = if ($destination != null) {
        $destination
    } else if ( $env.DEST_CLUSTER? != null ) {
        $env.DEST_CLUSTER
    } else {
        cb-env | get cluster
    }
    
    let params = {
        src : $src_cluster,
        src_bucket: $src_bucket,
        src_scope: $src_scope,
        src_collection: $src_collection,
        dest: $dest_cluster,
        dest_bucket: $dest_bucket,
        dest_scope: $dest_scope,
        dest_collection: $dest_collection,
    }
    do $operation $params
}
