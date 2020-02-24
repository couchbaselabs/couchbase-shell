 %{
     // to make this grammar similar to the golang N1QL grammar, we need to implement some of the convenience functions
     // in golang that are used in the parser productions.
     
     function expr(type,ex) {
	       this.type = type;
	       this.ops = {};
	       //console.log("Creating expression type: " + type + (ex ? (" (" + ex + ")") : ""));
     }

     expr.prototype.Alias = function() {return this.ops.name;};
     expr.prototype.Select = function() {return this.ops.select;};
     expr.prototype.Subquery = function() {return this.ops.subquery;};
     expr.prototype.Keys = function() {return this.ops.keys;};
     expr.prototype.Indexes = function() {return this.ops.indexes;};
     
     //
     // return all the fields found in the parse tree. Each field will be an array of terms
     //
     
     expr.prototype.getFields = function(fieldArray, aliases) {
	       //console.log("getting fields for item type: " + this.type);
		       
	       if (!fieldArray) fieldArray = [];
	       if (!aliases) aliases = {};
	       
	       switch (this.type) {
	       
	       // Subselect indicates a keyspace, and possibly an alias
	       case "Subselect": {
	         if (this.ops.from && this.ops.from.type == "KeyspaceTerm") {
	           if (this.ops.from.ops.keyspace)
	             fieldArray.push(this.ops.from.ops.keyspace);

               // if we see an alias, create a new alias object to included it	           
               if (this.ops.from.ops.as_alias) {
                 aliases = JSON.parse(JSON.stringify(aliases));
                 aliases[this.ops.from.ops.as_alias] = this.ops.from.ops.keyspace;
               }
	         }
	       }
	       break;
	       
           // if this has type "Field" or "Element", extract the path	       
	       case "Field":
	       case "Element": {
             var path = [];
             this.getFieldPath(path,fieldArray,aliases);
             if (path.length > 0)
                 fieldArray.push(path);
             
             break;
            }
             
           // any ExpressionTerm or ResultTerm can have an Identifier child that indicates
           // a field or bucket
           case "ExpressionTerm":
           case "ResultTerm":
             if (this.ops.expression && this.ops.expression.type == "Identifier")
                 fieldArray.push([this.ops.expression.ops.identifier]);
             break;

           // KeyspaceTerm gives bucket names in the from clause

           case "KeyspaceTerm":
             if (this.ops.keyspace)
                 fieldArray.push([this.ops.keyspace]);
             break;
           }

         // regardless, go through the "ops" object and call recursively on  our children
         for (var name in this.ops) {
             var child = this.ops[name];
             if (!child)
                 continue;
                 
             // if we are an array op, ignore the "mapping" and "when" fields
             if (this.type == "Array" && (name == "mapping" || name == "when"))
                 continue;
                 
             // the "satisfies" term for ANY, EVERY, etc., contains references to the bound variables,
             // and as such we can't find any useful field information             
             if (name == "satisfies")
                 continue;
                 
             // the "FIRST" operator has an expression based on bindings, which we must ignore
             if (this.type == "First" && (name == "expression" || name == "when"))
                 continue;
             
                 
             
             //console.log("  got child: " + name + "(" + (child.type && child.ops) + ") = " + JSON.stringify(child));
             
             if (child.getFields)  {
                 //console.log("  got child type: " + child.type);
                 child.getFields(fieldArray,aliases);
             }

             // some children are arrays
             else if (child.length) for (var i=0; i< child.length; i++) if (child[i] && child[i].getFields) {
                 //console.log("  got child[" + i + "] type: " + child[i].type);
                 child[i].getFields(fieldArray,aliases);
             }
         }
     };
     
     //
     // if we have a field, we can build its list of path elements
     // Field expressions come in a variety of forms
     //   - "Field" -> "Identifier" (first item in path), "FieldName" (next item in path) 
     //   - "Element" -> "Field" (array expr prefix), expr (array expression)
     // 
     // We expect currentPath to be an array into which we put the elements in the path
     // 
     
     expr.prototype.getFieldPath = function(currentPath,fieldArray,aliases) {
	       //console.log("Getting field path for type: " + this.type);
         // error checking: must have ops
         if (!this.ops)
             return;

         // Field type - first might be Identifier, first element in path
         //            - might be Element, meaning array expression
         //  first might also be Field, needing recursive call
         //  second is usually next item in path
         
         if ((this.type == "Field" || this.type == "Element") && this.ops.first) {
             if (this.ops.first.type == "Identifier") {
                 var id = this.ops.first.ops.identifier; // if the first element is an alias, resolve it
                 if (aliases && aliases[id])
                     id = aliases[id];
                 currentPath.push(id);
             }
             else if (this.ops.first.type == "Field" || this.ops.first.type == "Element")
                 this.ops.first.getFieldPath(currentPath,fieldArray,aliases);
         }

         else if (this.type == "Identifier" && this.ops.identifier) {
             currentPath.push(this.ops.identifier);
         }
         
         else if (this.type == "FieldName" && this.ops.field_name) {
             currentPath.push(this.ops.identifier);
         }

         // if we have type "Field", the "second" field may be part of the path expression
         
         if (this.type == "Field" && this.ops.second && this.ops.second.type == "FieldName")
             currentPath.push(this.ops.second.ops.field_name);
         
         // if we have type "Element", second is unconnected expression that should 
         // none-the-less be scanned for other field names
         
         if (this.type == "Element" && this.ops.second.getFields) {
             if (currentPath.length > 0)
                 currentPath.push("[]"); // indicate the array reference in the path
             this.ops.second.getFields(fieldArray);
         }
     };
     

     var expression = {};
     expression.Bindings = [];
     expression.Expressions = [];
     expression.FALSE_EXPR = "FALSE";
     expression.MISSING_EXPR = "MISSING";
     expression.NULL_EXPR = "NULL";
     expression.TRUE_EXPR = "TRUE";
     
     expression.NewAdd = function(first, second)                     {var e = new expr("Add"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewAll = function(all_expr, distinct)                {var e = new expr("All"); e.ops.all_expr = all_expr; return e;};
     expression.NewAnd = function(first, second)                     {var e = new expr("And"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewAny = function(bindings, satisfies)               {var e = new expr("Any"); e.ops.bindings = bindings; e.ops.satisfies = satisfies; return e;};
     expression.NewAnyEvery = function(bindings, satisfies)          {var e = new expr("AnyEvery"); e.ops.bindings = bindings; e.ops.satisfies = satisfies;return e;};
     expression.NewArray = function(mapping, bindings, when)         {var e = new expr("Array"); e.ops.mapping = mapping; e.ops.bindings = bindings; e.ops.when = when; return e;};
     expression.NewArrayConstruct = function(elements)               {var e = new expr("ArrayConstruct"); e.ops.elements = elements; return e;};
     expression.NewArrayStar = function(operand)                     {var e = new expr("ArrayStar"); e.ops.operand = operand; return e;};
     expression.NewBetween = function(item, low, high)               {var e = new expr("Between"); e.ops.item = item; e.ops.low = low; e.ops.high = high; return e;};
     expression.NewBinding = function(name_variable, variable, binding_expr, descend)
     {var e = new expr("Binding"); e.ops.name_variable = name_variable; e.ops.variable = variable; e.ops.binding_expr = binding_expr; e.ops.descend = descend; return e;};
     expression.NewConcat = function(first, second)                  {var e = new expr("Concat"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewConstant = function(value)                        {var e = new expr("Constant"); e.ops.value = value; return e;};
     expression.NewCover = function(covered)                         {var e = new expr("Cover"); e.ops.covered = covered; return e;};
     expression.NewDiv = function(first, second)                     {var e = new expr("Div"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewElement = function(first, second)                 {var e = new expr("Element"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewEq = function(first, second)                      {var e = new expr("Eq"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewEmpty = function()                                {var e = new expr("Empty"); return e;};
     expression.NewEvery = function(bindings, satisfies)             {var e = new expr("Every"); e.ops.bindings = bindings; e.ops.satisfies = satisfies; return e;};
     expression.NewExists = function(operand)                        {var e = new expr("Exists"); e.ops.operand = operand; return e;};
     expression.NewField = function(first,second)                    {var e = new expr("Field"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewFieldName = function(field_name,case_insensitive) {var e = new expr("FieldName",field_name); e.ops.field_name = field_name; e.ops.case_insensitive = case_insensitive; return e;};
     expression.NewFirst = function(expression,coll_bindings,when)   {var e = new expr("First"); e.ops.expression = expression; e.ops.coll_bindings = coll_bindings; e.ops.when = when; return e;};
     expression.NewGE = function(first, second)                      {var e = new expr("GE"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewGT = function(first, second)                      {var e = new expr("GT"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewIdentifier = function(identifier)                 {var e = new expr("Identifier",identifier); e.ops.identifier = identifier; return e;};
     expression.NewIn = function(first, second)                      {var e = new expr("In"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewIsMissing = function(operand)                     {var e = new expr("IsMissing"); e.ops.operand = operand; return e;};
     expression.NewIsNotNull = function(operand)                     {var e = new expr("IsNotNull"); e.ops.operand = operand; return e;};
     expression.NewIsNotMissing = function(operand)                  {var e = new expr("IsNotMissing"); e.ops.operand = operand; return e;};
     expression.NewIsNotValued = function(operand)                   {var e = new expr("IsNotValued"); e.ops.operand = operand; return e;};
     expression.NewIsNull = function(operand)                        {var e = new expr("IsNull"); e.ops.operand = operand; return e;};
     expression.NewIsValued = function(operand)                      {var e = new expr("IsValued"); e.ops.operand = operand; return e;};
     expression.NewLE = function(first, second)                      {var e = new expr("LE"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewLT = function(first, second)                      {var e = new expr("LT"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewLike = function(first, second)                    {var e = new expr("Like"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewMod = function(first, second)                     {var e = new expr("Mod"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewMult = function(first, second)                    {var e = new expr("Multi"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewNE = function(first, second)                      {var e = new expr("NE"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewNeg = function(operand)                           {var e = new expr("Neg"); e.ops.operand = operand; return e;};
     expression.NewNot = function(operand)                           {var e = new expr("Not"); e.ops.operand = operand; return e;};
     expression.NewNotBetween = function(iteem, low, high)           {var e = new expr("NotBetween"); e.ops.item = item; e.ops.low = low; e.ops.high = high; return e;};
     expression.NewNotIn = function(first, second)                   {var e = new expr("NotIn"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewNotLike = function(first, second)                 {var e = new expr("NotLike"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewNotWithin = function(first, second)               {var e = new expr("NotWithin"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewObject = function(name_mapping, value_mapping, bindings, when)
     {var e = new expr("Object"); e.ops.name_mapping = name_mapping; e.ops.value_mapping = value_mapping; e.ops.bindings = bindings; e.ops.when = when; return e;};
     expression.NewObjectConstruct = function(mapping)               {var e = new expr("ObjectConstruct"); e.ops.mapping = mapping; return e;};
     expression.NewOr = function(first, second)                      {var e = new expr("Or"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewSearchedCase = function(when_terms, else_term)    {var e = new expr("SearchedCase"); e.ops.when_terms = when_terms; e.ops.else_term = else_term; return e;};
     expression.NewSelf = function()                                 {var e = new expr("Self"); return e;};
     expression.NewSimpleBinding = function(variable, binding_expr)  {var e = new expr("SimpleBinding"); e.ops.variable = variable; e.ops.binding_expr = binding_expr; return e;};
     expression.NewSimpleCase = function(search_term, when_terms, else_term)
     {var e = new expr("SimpleCase"); e.ops.search_term = search_term; e.ops.when_terms = when_terms; e.ops.else_term = else_term; return e;};
     expression.NewSlice = function(first, second, third)            {var e = new expr("Slice"); e.ops.first = first; e.ops.second = second; e.ops.third = third; return e;};
     expression.NewFunction = function(fname, param_expr, distinct)  {var e = new expr("Function"); e.ops.fname = fname; e.ops.param_expr = param_expr; e.ops.distinct = distinct; return e;};
     expression.NewSub = function(first, second)                     {var e = new expr("Sub"); e.ops.first = first; e.ops.second = second; return e;};
     expression.NewWithin = function(first, second)                  {var e = new expr("Within"); e.ops.first = first; e.ops.second = second; return e;};

     //

     var algebra = {};
     algebra.EMPTY_USE = new expr("EMPTY_USE");
     algebra.GetAggregate = function(name, dummy, has_window)                {var a = new expr("Aggregate"); a.ops.name = name; return a;}
     algebra.MapPairs = function(pairs)                                      {var a = new expr("Pairs"); a.ops.pairs = pairs; return a;}
     algebra.NewAdvise = function(statement)                                 {var a = new expr("Advise"); a.ops.statement = statement; return a;};
     algebra.NewAlterIndex = function(keyspace, index_name, opt_using, rename){var a = new expr("AlterIndex"); a.ops.keyspace = keyspace; a.ops.index_name = index_name; a.ops.opt_using = opt_using; a.ops.rename = rename; return a;};
     algebra.NewAnsiJoin = function(from,join_type,join_term,for_ident)      {var a = new expr("AnsiJoin"); a.ops.from = from; a.ops.join_type = join_type; a.ops.join_term = join_term; a.ops.for_ident = for_ident; return a;};
     algebra.NewAnsiNest = function(from,join_type,join_term,for_ident)      {var a = new expr("AnsiNest"); a.ops.from = from; a.ops.join_type = join_type; a.ops.join_term = join_term; a.ops.for_ident = for_ident; return a;};
     algebra.NewAnsiRightJoin = function(keyspace,join_term,for_ident)       {var a = new expr("AnsiRightJoin"); a.ops.ks = keyspace; a.ops.join_term = join_term; a.ops.for_ident = for_ident; return a;};
     algebra.NewBuildIndexes = function(keyspace,opt_index,index_names)      {var a = new expr("BuildIndexes"); a.ops.keyspace = keyspace; a.opt_index = opt_index; a.ops.index_names = index_names; return a;};
     algebra.NewCreateFunction = function(name,body,params)                  {var a = new expr("CreateFunction"); a.ops.name = name; a.ops.body = body; a.ops.params = params;}
     algebra.NewCreateIndex = function(index_name,keyspace,index_terms,index_partition,index_where,index_using,index_with) 
       {var a = new expr("CreateIndex"); 
       a.ops.index_name = index_name; 
       a.ops.keyspace = keyspace; 
       a.ops.index_terms = index_terms; 
       a.ops.index_partition = index_partition; 
       a.ops.index_where = index_where; 
       a.ops.index_using = index_using; 
       a.ops.index_where = index_where; return a;};
     algebra.NewCreatePrimaryIndex = function(opt_name,keyspace,index_using,index_with) {var a = new expr("CreatePrimateIndex"); a.ops.opt_name = opt_name; a.ops.keyspace = keyspace; a.ops.index_using = index_using; a.ops.index_with = index_with; return a;};
     algebra.NewDelete = function(keyspace,opt_use_keys,opt_use_indexes,opt_where,opt_limit,opt_returning) {var a = new expr("Delete"); a.ops.keyspace = keyspace; a.ops.opt_use_keys = opt_use_keys; a.ops.opt_use_indexes = opt_use_indexes; a.ops.opt_where = opt_where; a.ops.opt_limit = opt_limit; return a;};
     algebra.NewDropFunction = function(name)                                 {var a = new expr("DropFunction"); a.ops.name = name; return a;};
     algebra.NewDropIndex = function(keyspace, opt_using)                     {var a = new expr("DropIndex"); a.ops.keyspace = keyspace; a.ops.opt_using = opt_using; return a;};
     algebra.NewExcept = function(first,except)                               {var a = new expr("Except"); a.ops.first = first; a.ops.except = except; return a;};
     algebra.NewExceptAll = function(first,except)                            {var a = new expr("ExceptAll"); a.ops.first = first; a.ops.except = except; return a;};
     algebra.NewExecute = function(expression,using)                          {var a = new expr("Execute"); a.ops.expression = expression; a.ops.using = using; return a;};
     algebra.NewExecuteFunction = function(name,expression)                   {var a = new expr("ExecuteFunction"); a.ops.expression = expression; a.ops.name = name; return a;};
     algebra.NewExplain = function(statement)                                 {var a = new expr("Explain"); a.ops.statement = statement; return a;};
     algebra.NewExpressionTerm = function(expression, opt_as_alias, opt_use)  {var a = new expr("ExpressionTerm"); a.ops.expression = expression; a.ops.opt_as_alias = opt_as_alias; a.ops.opt_use = opt_use; return a;};
     algebra.NewGrantRole = function(role_list,user_list,keyspace_list)       {var a = new expr("GrantRole"); a.ops.role_list = role_list; a.ops.user_list = user_list; a.ops.keyspace_list = keyspace_list; return a;};
     algebra.NewGroup = function(expression,opt_letting,opt_having)           {var a = new expr("Group"); a.ops.expression = expression; a.ops.opt_letting = opt_letting; a.ops.opt_having = opt_having; return a;};
     algebra.NewGroupTerm = function(expression,opt_as_alias)                 {var a = new expr("GroupTerm"); a.ops.expression = expression; a.ops.opt_as_alias = opt_as_alias; return a;};
     algebra.NewIndexJoin = function(from,join_type,join_term,for_ident)      {var a = new expr("IndexJoin"); a.ops.from = from; a.ops.join_type = join_type; a.ops.join_term = join_term; a.ops.for_ident = for_ident; return a;};
     algebra.NewIndexKeyTerm = function(index_term,opt_dir)                   {var a = new expr("IndexKeyTerm"); a.ops.index_term = index_term; a.ops.opt_dir = opt_dir; return a;};
     algebra.NewIndexNest = function(from,join_type,join_term,for_ident)      {var a = new expr("IndexNest"); a.ops.from = from; a.ops.join_type = join_type; a.ops.join_term = join_term; a.ops.for_ident = for_ident; return a;};
     algebra.NewIndexRef = function(index_name,opt_using)                     {var a = new expr("IndexRef"); a.ops.index_name = index_name; a.ops.opt_using = opt_using; return a;};
     algebra.NewInferKeyspace = function(keyspace,infer_using,infer_with)     {var a = new expr("InferKeyspace"); a.ops.keyspace = keyspace; a.ops.infer_using = infer_using; a.ops.infer_with = infer_with; return a;};
     algebra.NewInsertSelect = function(keyspace,key_expr,value_expr,fullselect,returning) {var a = new expr("InsertSelect"); a.ops.keyspace = keyspace; a.ops.key_expr = key_expr; a.ops.value_expr = value_expr; return a;};
     algebra.NewInsertValues = function(keyspace,values_header,values_list,returning) {var a = new expr("InsertValues"); a.ops.values_header = values_header, a.ops.values_list = values_list; a.ops.returning = returning; return a;};
     algebra.NewIntersect = function(select_terms,intersect_term)             {var a = new expr("Intersect"); a.ops.elect_terms = elect_terms; a.ops.intersect_term = intersect_term; return a;};
     algebra.NewIntersectAll = function(select_terms,intersect_term)          {var a = new expr("IntersectAll"); a.ops.select_terms = select_terms; a.ops.intersect_term = intersect_term; return a;};
     algebra.NewJoin = function(from,join_type,join_term)                     {var a = new expr("Join"); a.ops.from = from; a.ops.join_type = join_type; a.ops.join_term = join_term; return a;};
     algebra.NewKeyspaceRef = function(namespace,keyspace,alias)              {var a = new expr("KeyspaceRef"); a.ops.namespace = namespace; a.ops.keyspace = keyspace; a.ops.alias = alias; return a;};
     algebra.NewKeyspaceTerm = function(namespace,keyspace,as_alias,opt_use)  {var a = new expr("KeyspaceTerm"); a.ops.namespace = namespace; a.ops.keyspace = keyspace; a.ops.as_alias = as_alias; a.ops.opt_use = opt_use; return a;};
     algebra.NewKeyspaceTermFromPath = function(path,as_alias,opt_use_keys,opt_use_indexes)  {var a = new expr("KeyspaceTermFromPath"); a.ops.path = path; a.ops.as_alias = as_alias; a.ops.opt_use_keys = opt_use_keys; a.ops.opt_use_indexes = opt_use_indexes; return a;};
     algebra.NewMerge = function(keyspace,merge_source,key,merge_actions,opt_limit,returning) {var a = new expr("Merge"); a.ops.keyspace = keyspace; a.ops.merge_source = merge_source; a.ops.key = key; a.ops.merge_actions = merge_actions; a.ops.opt_limit = opt_limit; a.ops.returning = returning; return a;};
     algebra.NewMergeActions = function(update,del,insert)                    {var a = new expr("MergeActions"); a.ops.update = update; a.ops.del = del; a.ops.insert = insert; return a;};
     algebra.NewMergeDelete = function(where)                                 {var a = new expr("MergeDelete"); a.ops.where = where; return a;};
     algebra.NewMergeInsert = function(key_expr,expression,where)             {var a = new expr("MergeInsert"); a.ops.key_expr = key_expr;  a.ops.expression = expression; a.ops.where = where; return a;};
     algebra.NewMergeSourceExpression = function(expression,alias)            {var a = new expr("MergeSourceSelect"); a.ops.expression = expression; a.ops.alias = alias; return a;};
     algebra.NewMergeSourceFrom = function(from,alias)                        {var a = new expr("MergeSourceSelect"); a.ops.from = from; a.ops.alias = alias; return a;};
     algebra.NewMergeSourceSelect = function(from,alias)                      {var a = new expr("MergeSourceSelect"); a.ops.from = from; a.ops.alias = alias; return a;};
     algebra.NewMergeUpdate = function(set,unset,where)                       {var a = new expr("MergeUpdate"); a.ops.set = set; a.ops.unset = unset; a.ops.where = where; return a;};
     algebra.NewNamedParameter = function(named_param)                        {var a = new expr("NamedParameter"); a.ops.named_param = named_param; return a;};
     algebra.NewNest = function(from,join_type,join_term)                     {var a = new expr("Nest"); a.ops.from = from; a.ops.join_type = join_type; a.ops.join_term = join_term; return a;};
     algebra.NewOrder = function(sort_terms)                                  {var a = new expr("Order"); a.ops.sort_terms = sort_terms; return a;};
     algebra.NewOrderNulls = function(do_nulls, do_nulls2, last)              {var a = new expr("Order"); a.ops.do_nulls = do_nulls; a.ops.do_nulls2 = do_nulls2; a.ops.last = last; return a;};
     algebra.NewOrderNullsPos = function(dir,nulls)                           {var a = new expr("Order"); a.ops.dir = dir; a.ops.nulls = nulls; return a;};
     algebra.NewPair = function(first,second)                                 {var a = new expr("Pair"); a.ops.first = first; a.ops.second = second; return a;};
     algebra.NewPathLong = function(namespace,bucket,scope,keyspace)          {var a = new expr("PathShort"); a.ops.namespace = namespace; a.ops.keyspace = keyspace; a.ops.bucket = bucket; a.ops.scope = scope; return a;};
     algebra.NewPathShort = function(namespace,keyspace)                      {var a = new expr("PathShort"); a.ops.namespace = namespace; a.ops.keyspace = keyspace; return a;};
     algebra.NewPositionalParameter = function(positional_param)              {var a = new expr("PositionalParameter"); a.ops.positional_param = positional_param; return a;};
     algebra.NewPrepare = function(name,statement)                            {var a = new expr("Prepare"); a.ops.name = name; a.ops.statement = statement; return a;};
     algebra.NewProjection = function(distinct,projects)                      {var a = new expr("Projection"); a.ops.distinct = distinct; a.ops.projects = projects; return a;};
     algebra.NewRawProjection = function(distinct,expression,as_alias)        {var a = new expr("RawProjection"); a.ops.distinct = distinct; a.ops.expression = expression; a.ops.as_alias = as_alias; return a;};
     algebra.NewResultTerm = function(expression,star,as_alias)               {var a = new expr("ResultTerm"); a.ops.expression = expression; a.ops.star = star; a.ops.as_alias = as_alias; return a;};
     algebra.NewRevokeRule = function(role_list,user_list,keyspace_list)      {var a = new expr("RevokeRule"); a.ops.role_list = role_list; a.ops.user_list = user_list; a.ops.keyspace_list = keyspace_list; return a;};
     algebra.NewSelect = function(select_terms,order_by,offset,limit)         {var a = new expr("Select"); a.ops.select_terms = select_terms; a.ops.order_by = order_by; a.ops.offset = offset; a.ops.limit = limit; return a;};
     algebra.NewSelectTerm = function(term)                                   {var a = new expr("SelectTerm"); a.ops.term = term; return a;};
     algebra.NewSet = function(set_terms)                                     {var a = new expr("Set"); a.ops.set_terms = set_terms; return a;};
     algebra.NewSetTerm = function(path,expression,update_for)                {var a = new expr("SetTerm"); a.ops.path = path; a.ops.expression = expression; a.ops.update_for = update_for; return a;};
     algebra.NewSortTerm = function(expression,desc,order_nulls_pos)          {var a = new expr("SortTerm"); a.ops.expression = expression; a.ops.desc = desc; a.order_nulls_pos = order_nulls_pos; return a;};
     algebra.NewSubquery = function(fullselect)                               {var a = new expr("Subquery"); a.ops.fullselect = fullselect; return a;};
     algebra.NewSubqueryTerm = function(select_term,as_alias)                 {var a = new expr("SubqueryTerm"); a.ops.select_term = select_term; a.ops.as_alias = as_alias; return a;};
     algebra.NewSubselect = function(with_expr,from,let,where,group,select)   {var a = new expr("Subselect"); a.ops.with_expr = with_expr; a.ops.from = from; a.ops.let = let; a.ops.where = where; a.ops.group = group; a.ops.select = select; return a;};
     algebra.NewUnion = function(first,second)                                {var a = new expr("Union"); a.ops.first = first; a.ops.second = second; return a;};
     algebra.NewUnionAll = function(first,second)                             {var a = new expr("UnionAll"); a.ops.first = first; a.ops.second = second; return a;};
     algebra.NewUnnest = function(from,join_type,expression,as_alias)         {var a = new expr("Unnest"); a.ops.from = from; a.ops.join_type = join_type; a.ops.expression = expression; a.ops.as_alias = as_alias; return a;};
     algebra.NewUnset = function(unset_terms)                                 {var a = new expr("Unset"); a.ops.unset_terms = unset_terms; return a;};
     algebra.NewUnsetTerm = function(path,update_for)                         {var a = new expr("UnsetTerm"); a.ops.path = path; a.ops.update_for = update_for; return a;};
     algebra.NewUpdate = function(keyspace,use_keys,use_indexes,set,unset,where,limit,returning) {var a = new expr("Update"); a.ops.keyspace = keyspace; a.ops.use_keys = use_keys; a.ops.use_indexes = use_indexes; a.ops.set = set; a.ops.unset = unset; a.ops.where = where; a.ops.limit = limit; a.ops.returning = returning; return a;};
     algebra.NewUpdateFor = function(update_dimensions,when)                  {var a = new expr("UpdateFor"); a.ops.update_dimensions = update_dimensions; a.ops.when = when; return a;};
     algebra.NewUpdateStatistics = function(keyspace,terms,with_expr)         {var a = new expr("UpdateStatistics"); a.ops.keyspace = keyspace; a.ops.terms = terms; a.ops.with_expr = with_expr; return a;};
     algebra.NewUpsertSelect = function(keyspace,key_expr,value_expr,fullselect,returning) {var a = new expr("UpsertSelect"); a.ops.keyspace = keyspace; a.ops.key_expr = key_expr; a.ops.value_expr = value_expr; a.ops.fullselect = fullselect; a.ops.returning = returning; return a;};
     algebra.NewUpsertValues = function(keyspace,values_list,returning)       {var a = new expr("UpsertValues"); a.ops.keyspace = keyspace; a.ops.values_list = values_list; a.ops.returning = returning; return a;};
     algebra.NewUse = function(keys,index, hint)                              {var a = new expr("Use"); a.ops.keys = keys; a.ops.index = index; a.ops.hint = hint; 
                                                                               a.SetKeys = function(keys) {a.ops.keys = keys;}; a.SetIndexes = function(indexes) {a.ops.index = indexes;}; a.SetJoinHint = function(hint) {a.ops.hint=hint}; 
                                                                               a.Indexes = function() {return a.ops.index}; a.JoinHint = function() {return a.ops.hint}; a.Keys = function() {return a.ops.keys};
                                                                               return a;};
     algebra.NewWindowTerm = function(partition, order, frame)                {var a = new expr("WindowTerm"); a.ops.partition = partition; a.ops.order = order; a.ops.frame = frame; return a;};
     algebra.NewWindowFrame = function(modifier, extents)                     {var a = new expr("WindowFrame"); a.ops.modifier = modifier; a.ops.extents = extents; return a;};
     algebra.NewWindowFrameExtent = function(exprn, extent)                   {var a = new expr("WindowFrameExtent"); a.ops.exprn = exprn; a.ops.extent = extent; return a;};
     algebra.WindowFrameExtents = function(from, to)                          {var a = new expr("WindowFrameExtents"); a.ops.from = from; a.ops.to = to; return a;};

     algebra.SubqueryTerm = "SubqueryTerm";
     algebra.ExpressionTerm = "ExpressionTerm";
     algebra.KeyspaceTerm = "KeyspaceTerm";
     
     algebra.AGGREGATE_FROMLAST = "AGGREGATE_FROMLAST";
     algebra.AGGREGATE_FROMFIRST = "AGGREGATE_FROMFIRST";
     algebra.AGGREGATE_DISTINCT = "AGGREGATE_DISTINCT";
     algebra.AGGREGATE_RESPECTNULLS = "AGGREGATE_RESPECTNULLS";
     algebra.AGGREGATE_IGNORENULLS = "AGGREGATE_IGNORENULLS";

     algebra.WINDOW_FRAME_ROWS = "WINDOW_FRAME_ROWS";
     algebra.WINDOW_FRAME_RANGE = "WINDOW_FRAME_RANGE";
     algebra.WINDOW_FRAME_GROUPS = "WINDOW_FRAME_GROUPS";
     algebra.WINDOW_FRAME_EXCLUDE_CURRENT_ROW = "WINDOW_FRAME_EXCLUDE_CURRENT_ROW";
     algebra.WINDOW_FRAME_EXCLUDE_TIES = "WINDOW_FRAME_EXCLUDE_TIES";
     algebra.WINDOW_FRAME_EXCLUDE_GROUP = "WINDOW_FRAME_EXCLUDE_GROUP";
     algebra.WINDOW_FRAME_UNBOUNDED_PRECEDING = "WINDOW_FRAME_UNBOUNDED_PRECEDING";
     algebra.WINDOW_FRAME_UNBOUNDED_FOLLOWING = "WINDOW_FRAME_UNBOUNDED_FOLLOWING";
     algebra.WINDOW_FRAME_CURRENT_ROW = "WINDOW_FRAME_CURRENT_ROW";
     algebra.WINDOW_FRAME_VALUE_PRECEDING = "WINDOW_FRAME_VALUE_PRECEDING";
     algebra.WINDOW_FRAME_VALUE_FOLLOWING = "WINDOW_FRAME_VALUE_FOLLOWING";
     

     var value = {};
     value.NewValue = function(val) {var a = new expr("Value"); a.value = val; return a;};

     var datastore = {
         INF_DEFAULT : "INF_DEFAULT",
         DEFAULT : "DEFAULT",
         VIEW : "VIEW",
         GSI : "GSI",
         FTS : "FTS"    
     };
     
     var nil = null;

     var statement_count = 0;

     var yylex = {
         Error: function(message) {console.log(message);}
     };
%}

%lex

qidi                        [`](([`][`])|[^`])+[`][i]
qid                         [`](([`][`])|[^`])+[`]

%options flex case-insensitive

%%


\"((\\\")|[^\"])*\" { return 'STR'; }

\'(('')|[^\'])*\'   { return 'STR'; }

{qidi}              { yytext = yytext.substring(1,yytext.length -2).replace("``","`"); return 'IDENT_ICASE'; }

{qid}               { yytext = yytext.substring(1,yytext.length -1).replace("``","`"); return 'IDENT'; }
                                      
(0|[1-9][0-9]*)\.[0-9]+([eE][+\-]?[0-9]+)? { return 'NUM'; }

(0|[1-9][0-9]*)[eE][+\-]?[0-9]+ { return 'NUM';  }

0|[1-9][0-9]* { return 'NUM'; }

(\/\*)([^\*]|(\*)+[^\/])*((\*)+\/) /* eat up block comment */ 

"--"[^\n\r]*      /* eat up line comment */ 

[ \t\n\r\f]+      /* eat up whitespace */ 

"."               { return ("DOT"); }
"+"               { return ("PLUS"); }
"*"               { return ("STAR"); }
"/"               { return ("DIV"); }
"-"               { return ("MINUS"); }
"%"               { return ("MOD"); }
"=="      { return ("DEQ"); }
"="               { return ("EQ"); }
"!="      { return ("NE"); }
"<>"      { return ("NE"); }
"<"               { return ("LT"); }
"<="      { return ("LE"); }
">"               { return ("GT"); }
">="      { return ("GE"); }
"||"      { return ("CONCAT"); }
"("               { return ("LPAREN"); }
")"               { return ("RPAREN"); }
"{"               { return ("LBRACE"); }
"}"               { return ("RBRACE"); }
","               { return ("COMMA"); }
":"               { return ("COLON"); }
"["               { return ("LBRACKET"); }
"]"               { return ("RBRACKET"); }
"]i"      { return ("RBRACKET_ICASE"); }
";"               { return ("SEMI"); }
"!"               { return ("NOT_A_TOKEN"); }

<<EOF>>   { return 'EOF'; }

 
\$[a-zA-Z_][a-zA-Z0-9_]*   { return 'NAMED_PARAM'; }

\$[1-9][0-9]*              { return 'POSITIONAL_PARAM'; }

\?                         { return 'NEXT_PARAM'; }


"advise"                        { return("ADVISE"); }
"all"                           { return("ALL"); }
"alter"                         { return("ALTER"); }
"analyze"                       { return("ANALYZE"); }
"and"                           { return("AND"); }
"any"                           { return("ANY"); }
"array"                         { return("ARRAY"); }
"as"                            { return("AS"); }
"asc"                           { return("ASC"); }
"begin"                         { return("BEGIN"); }
"between"                       { return("BETWEEN"); }
"binary"                        { return("BINARY"); }
"boolean"                       { return("BOOLEAN"); }
"break"                         { return("BREAK"); }
"bucket"                        { return("BUCKET"); }
"build"                         { return("BUILD"); }
"by"                            { return("BY"); }
"call"                          { return("CALL"); }
"case"                          { return("CASE"); }
"cast"                          { return("CAST"); }
"cluster"                       { return("CLUSTER"); }
"collate"                       { return("COLLATE"); }
"collection"                    { return("COLLECTION"); }
"commit"                        { return("COMMIT"); }
"connect"                       { return("CONNECT"); }
"continue"                      { return("CONTINUE"); }
"correlated"                    { return("CORRELATED"); }
"cover"                         { return("COVER"); }
"create"                        { return("CREATE"); }
"current"                       { return("CURRENT"); }
"database"                      { return("DATABASE"); }
"dataset"                       { return("DATASET"); }
"datastore"                     { return("DATASTORE"); }
"declare"                       { return("DECLARE"); }
"decrement"                     { return("DECREMENT"); }
"delete"                        { return("DELETE"); }
"derived"                       { return("DERIVED"); }
"desc"                          { return("DESC"); }
"describe"                      { return("DESCRIBE"); }
"distinct"                      { return("DISTINCT"); }
"do"                            { return("DO"); }
"drop"                          { return("DROP"); }
"each"                          { return("EACH"); }
"element"                       { return("ELEMENT"); }
"else"                          { return("ELSE"); }
"end"                           { return("END"); }
"every"                         { return("EVERY"); }
"except"                        { return("EXCEPT"); }
"exclude"                       { return("EXCLUDE"); }
"execute"                       { return("EXECUTE"); }
"exists"                        { return("EXISTS"); }
"explain"                       { return("EXPLAIN") }
"false"                         { return("FALSE"); }
"fetch"                         { return("FETCH"); }
"first"                         { return("FIRST"); }
"flatten"                       { return("FLATTEN"); }
"following"                     { return("FOLLOWING"); }
"for"                           { return("FOR"); }
"force"                         { return("FORCE"); }
"from"                          { return("FROM"); }
"fts"                           { return("FTS"); }
"function"                      { return("FUNCTION"); }
"golang"                        { return("GOLANG"); }
"grant"                         { return("GRANT"); }
"group"                         { return("GROUP"); }
"groups"                        { return("GROUPS"); }
"gsi"                           { return("GSI"); }
"hash"                          { return("HASH"); }
"having"                        { return("HAVING"); }
"if"                            { return("IF"); }
"ignore"                        { return("IGNORE"); }
"ilike"                         { return("ILIKE"); }
"in"                            { return("IN"); }
"include"                       { return("INCLUDE"); }
"increment"                     { return("INCREMENT"); }
"index"                         { return("INDEX"); }
"infer"                         { return("INFER"); }
"inline"                        { return("INLINE"); }
"inner"                         { return("INNER"); }
"insert"                        { return("INSERT"); }
"intersect"                     { return("INTERSECT"); }
"into"                          { return("INTO"); }
"is"                            { return("IS"); }
"javascript"                    { return("JAVASCRIPT"); }
"join"                          { return("JOIN"); }
"key"                           { return("KEY"); }
"keys"                          { return("KEYS"); }
"keyspace"                      { return("KEYSPACE"); }
"known"                         { return("KNOWN"); }
"language"                      { return("LANGUAGE"); }
"last"                          { return("LAST"); }
"left"                          { return("LEFT"); }
"let"                           { return("LET"); }
"letting"                       { return("LETTING"); }
"like"                          { return("LIKE"); }
"limit"                         { return("LIMIT"); }
"lsm"                           { return("LSM"); }
"map"                           { return("MAP"); }
"mapping"                       { return("MAPPING"); }
"matched"                       { return("MATCHED"); }
"materialized"                  { return("MATERIALIZED"); }
"merge"                         { return("MERGE"); }
"minus"                         { return("MINUS"); }
"missing"                       { return("MISSING"); }
"namespace"                     { return("NAMESPACE"); }
"namespace_id"                  { return("NAMESPACE_ID"); }
"nest"                          { return("NEST"); }
"nl"                            { return("NL"); }
"no"                            { return("NO"); }
"not"                           { return("NOT"); }
"not_a_token"                   { return("NOT_A_TOKEN"); }
"nth_value"                     { return("NTH_VALUE"); }
"null"                          { return("NULL"); }
"nulls"                         { return("NULLS"); }
"number"                        { return("NUMBER"); }
"object"                        { return("OBJECT"); }
"offset"                        { return("OFFSET"); }
"on"                            { return("ON"); }
"option"                        { return("OPTION"); }
"or"                            { return("OR"); }
"order"                         { return("ORDER"); }
"others"                        { return("OTHERS"); }
"outer"                         { return("OUTER"); }
"over"                          { return("OVER"); }
"parse"                         { return("PARSE"); }
"partition"                     { return("PARTITION"); }
"password"                      { return("PASSWORD"); }
"path"                          { return("PATH"); }
"pool"                          { return("POOL"); }
"preceding"                     { return("PRECEDING") }
"prepare"                       { return("PREPARE") }
"primary"                       { return("PRIMARY"); }
"private"                       { return("PRIVATE"); }
"privilege"                     { return("PRIVILEGE"); }
"probe"                         { return("PROBE"); }
"procedure"                     { return("PROCEDURE"); }
"public"                        { return("PUBLIC"); }
"range"                         { return("RANGE"); }
"raw"                           { return("RAW"); }
"realm"                         { return("REALM"); }
"reduce"                        { return("REDUCE"); }
"rename"                        { return("RENAME"); }
"respect"                       { return("RESPECT"); }
"return"                        { return("RETURN"); }
"returning"                     { return("RETURNING"); }
"revoke"                        { return("REVOKE"); }
"right"                         { return("RIGHT"); }
"role"                          { return("ROLE"); }
"rollback"                      { return("ROLLBACK"); }
"row"                           { return("ROW"); }
"rows"                          { return("ROWS"); }
"satisfies"                     { return("SATISFIES"); }
"schema"                        { return("SCHEMA"); }
"select"                        { return("SELECT"); }
"self"                          { return("SELF"); }
"semi"                          { return("SEMI"); }
"set"                           { return("SET"); }
"show"                          { return("SHOW"); }
"some"                          { return("SOME"); }
"start"                         { return("START"); }
"statistics"                    { return("STATISTICS"); }
"string"                        { return("STRING"); }
"system"                        { return("SYSTEM"); }
"then"                          { return("THEN"); }
"ties"                          { return("TIES"); }
"to"                            { return("TO"); }
"transaction"                   { return("TRANSACTION"); }
"trigger"                       { return("TRIGGER"); }
"true"                          { return("TRUE"); }
"truncate"                      { return("TRUNCATE"); }
"unbounded"                     { return("UNBOUNDED"); }
"under"                         { return("UNDER"); }
"union"                         { return("UNION"); }
"unique"                        { return("UNIQUE"); }
"unknown"                       { return("UNKNOWN"); }
"unnest"                        { return("UNNEST"); }
"unset"                         { return("UNSET"); }
"update"                        { return("UPDATE"); }
"upsert"                        { return("UPSERT"); }
"use"                           { return("USE"); }
"user"                          { return("USER"); }
"using"                         { return("USING"); }
"validate"                      { return("VALIDATE"); }
"value"                         { return("VALUE"); }
"valued"                        { return("VALUED"); }
"values"                        { return("VALUES"); }
"via"                           { return("VIA"); }
"view"                          { return("VIEW"); }
"when"                          { return("WHEN"); }
"where"                         { return("WHERE"); }
"while"                         { return("WHILE"); }
"with"                          { return("WITH"); }
"within"                        { return("WITHIN"); }
"work"                          { return("WORK"); }
"xor"                           { return("XOR"); }

[a-zA-Z_][a-zA-Z0-9_]*     { return 'IDENT'; }

/lex

/* Precedence: lowest to highest */
%left           ORDER
%left           UNION INTERESECT EXCEPT
%left           JOIN NEST UNNEST FLATTEN INNER LEFT RIGHT
%left           OR
%left           AND
%right          NOT
%nonassoc       EQ DEQ NE
%nonassoc       LT GT LE GE
%nonassoc       LIKE
%nonassoc       BETWEEN
%nonassoc       IN WITHIN
%nonassoc       EXISTS
%nonassoc       IS                              /* IS NULL, IS MISSING, IS VALUED, IS NOT NULL, etc. */
%left           CONCAT
%left           PLUS MINUS
%left           STAR DIV MOD

/* Unary operators */
%right          COVER
%left           ALL
%right          UMINUS
%left           DOT LBRACKET RBRACKET

/* Override precedence */
%left           LPAREN RPAREN
%start          input_list

/*****************************************************************************/
/*****************************************************************************/
/*****************************************************************************/

%%

input_list:
     inputs { /*console.log("Got input list: " + JSON.stringify($1));*/ return $1;}
;

inputs:
input EOF
{
    if ($1 && $1.getFields) {
        //console.log("Getting fields for: " + JSON.stringify($1,null,4));
        var fields = [];
        $1.getFields(fields);
        $1.pathsUsed = fields;
    }

    // ignore empty expressions
    if ($$.type == "Empty")
      $$ = [];
    else
      $$ = [$1];
}
|
input SEMI inputs
{
    if ($1 && $1.getFields) {
        var fields = [];
        $1.getFields(fields);
        $1.pathsUsed = fields;
    }

    // ignore empty expressions
    if ($$.type != "Empty")
      $3.push($1);
    $$ = $3;
}
;


input:
stmt_body
{
    $$ = $1;
    /*console.log("Got statement: " + JSON.stringify($1));*/
}
|
expr_input 
{
    $$ = $1;
    /*console.log("Got expression: " + JSON.stringify($1));*/
}
|
/* empty is o.k. */
{
    $$ = expression.NewEmpty();
}
;

/*opt_trailer:*/
/*{*/
  /* nothing */
/*}*/
/*|*/
/*opt_trailer SEMI*/
/*;*/

stmt_body:
advise
|
explain
|
prepare
|
execute
|
stmt
;

stmt:
select_stmt
|
dml_stmt
|
ddl_stmt
|
infer
|
update_statistics
|
role_stmt
|
function_stmt
;

advise:
ADVISE opt_index stmt
{
    $$ = algebra.NewAdvise($3)
}
;

opt_index:
/* empty */
|
INDEX
{
    /* yylex.(*lexer).setOffset($<tokOffset>1) */
}
;

explain:
EXPLAIN stmt
{
    $$ = algebra.NewExplain($2)
}
;

prepare:
PREPARE opt_force opt_name stmt
{
    $$ = algebra.NewPrepare($3, $4, $2)
}
;

opt_force:
/* empty */
{
    $$ = false
}
|
FORCE
{
    /*yylex.(*lexer).setOffset($<tokOffset>1)*/
    $$ = true
}
;

opt_name:
/* empty */
{
    $$ = ""
}
|
IDENT from_or_as
{
    $$ = $1
}
|
STR from_or_as
{
    $$ = $1
}
;

from_or_as:
FROM
{
    /*yylex.(*lexer).setOffset($<tokOffset>1)*/
}
|
AS
{
    /*yylex.(*lexer).setOffset($<tokOffset>1)*/
}
;

execute:
EXECUTE expr execute_using
{
    $$ = algebra.NewExecute($2, $3)
}
;

execute_using:
/* empty */
{
    $$ = nil
}
|
USING construction_expr
{
    $$ = $2
}
;

infer:
infer_keyspace
;

infer_keyspace:
INFER opt_keyspace keyspace_ref opt_infer_using opt_infer_ustat_with
{
    $$ = algebra.NewInferKeyspace($3, $4, $5)
}
;

opt_keyspace:
/* empty */
{
}
|
KEYSPACE
;

opt_infer_using:
/* empty */
{
    $$ = datastore.INF_DEFAULT
}
;

opt_infer_ustat_with:
/* empty */
{
    $$ = nil
}
|
infer_ustat_with
;

infer_ustat_with:
WITH expr
{
    $$ = $2;
    /*
    if $$ == nil {
    yylex.Error("WITH value must be static.")
    }
    */    
}
;

select_stmt:
fullselect
{
    $$ = $1
}
;

dml_stmt:
insert
|
upsert
|
delete
|
update
|
merge
;

ddl_stmt:
index_stmt
;

role_stmt:
grant_role
|
revoke_role
;

index_stmt:
create_index
|
drop_index
|
alter_index
|
build_index
;

function_stmt:
create_function
|
drop_function
|
execute_function
;

fullselect:
select_terms opt_order_by
{
    $$ = algebra.NewSelect($1, $2, nil, nil) /* OFFSET precedes LIMIT */
}
|
select_terms opt_order_by limit opt_offset
{
    $$ = algebra.NewSelect($1, $2, $4, $3) /* OFFSET precedes LIMIT */
}
|
select_terms opt_order_by offset opt_limit
{
    $$ = algebra.NewSelect($1, $2, $3, $4) /* OFFSET precedes LIMIT */
}
;

select_terms:
subselect
{
    $$ = $1
}
|
select_terms UNION select_term
{
    $$ = algebra.NewUnion($1, $3)
}
|
select_terms UNION ALL select_term
{
    $$ = algebra.NewUnionAll($1, $4)
}
|
select_terms INTERSECT select_term
{
    $$ = algebra.NewIntersect($1, $3)
}
|
select_terms INTERSECT ALL select_term
{
    $$ = algebra.NewIntersectAll($1, $4)
}
|
select_terms EXCEPT select_term
{
    $$ = algebra.NewExcept($1, $3)
}
|
select_terms EXCEPT ALL select_term
{
    $$ = algebra.NewExceptAll($1, $4)
}
|
subquery_expr UNION select_term
{
    var left_term = algebra.NewSelectTerm($1.Select())
    $$ = algebra.NewUnion(left_term, $3)
}
|
subquery_expr UNION ALL select_term
{
    var left_term = algebra.NewSelectTerm($1.Select())
    $$ = algebra.NewUnionAll(left_term, $4)
}
|
subquery_expr INTERSECT select_term
{
    var left_term = algebra.NewSelectTerm($1.Select())
    $$ = algebra.NewIntersect(left_term, $3)
}
|
subquery_expr INTERSECT ALL select_term
{
    var left_term = algebra.NewSelectTerm($1.Select())
    $$ = algebra.NewIntersectAll(left_term, $4)
}
|
subquery_expr EXCEPT select_term
{
    var left_term = algebra.NewSelectTerm($1.Select())
    $$ = algebra.NewExcept(left_term, $3)
}
|
subquery_expr EXCEPT ALL select_term
{
    var left_term = algebra.NewSelectTerm($1.Select())
    $$ = algebra.NewExceptAll(left_term, $4)
}
;

select_term:
subselect
{
    $$ = $1
}
|
subquery_expr
{
    $$ = algebra.NewSelectTerm($1.Select())
}
;

subselect:
from_select
|
select_from
;

from_select:
from opt_let opt_where opt_group select_clause
{
    $$ = algebra.NewSubselect(nil, $1, $2, $3, $4, $5)
}
|
opt_with from opt_let opt_where opt_group select_clause
{
    $$ = algebra.NewSubselect($1, $2, $3, $4, $5, $6)
}
;

select_from:
select_clause opt_from opt_let opt_where opt_group
{
    $$ = algebra.NewSubselect(nil, $2, $3, $4, $5, $1)
}
|
opt_with select_clause opt_from opt_let opt_where opt_group
{
    $$ = algebra.NewSubselect($1, $3, $4, $5, $6, $2)
}
;


/*************************************************
 *
 * SELECT clause
 *
 *************************************************/

select_clause:
SELECT
projection
{
    $$ = $2
}
;

projection:
opt_quantifier projects
{
    $$ = algebra.NewProjection($1, $2)
}
|
opt_quantifier raw expr opt_as_alias
{
    $$ = algebra.NewRawProjection($1, $3, $4)
}
;

opt_quantifier:
/* empty */
{ $$ = false }
|
ALL
{ $$ = false }
|
DISTINCT
{ $$ = true }
;

raw:
RAW
|
ELEMENT
|
VALUE
;

projects:
project
{
    $$ = [$1]
}
|
projects COMMA project
{
    $1.push($3);
    $$ = $1;
}
;

project:
STAR
{
    $$ = algebra.NewResultTerm(expression.SELF, true, "")
}
|
expr DOT STAR
{
    $$ = algebra.NewResultTerm($1, true, "")
}
|
expr opt_as_alias
{
    $$ = algebra.NewResultTerm($1, false, $2)
}
;

opt_as_alias:
/* empty */
{
    $$ = ""
}
|
as_alias
;

as_alias:
alias
|
AS alias
{
    $$ = $2
}
;

alias:
IDENT
;


/*************************************************
 *
 * FROM clause
 *
 *************************************************/

opt_from:
/* empty */
{
    $$ = nil
}
|
from
;

from:
FROM from_term
{
    $$ = $2
}
;

from_term:
simple_from_term
{
   /*
    if $1.JoinHint() != algebra.JOIN_HINT_NONE {
        yylex.Error(fmt.Sprintf("Join hint (USE HASH or USE NL) cannot be specified on the first from term %s", $1.Alias()))
    }
    */
    $$ = $1
}
|
from_term opt_join_type JOIN simple_from_term on_keys
{
    /*
    ksterm := algebra.GetKeyspaceTerm($4)
    if ksterm == nil {
        yylex.Error("JOIN must be done on a keyspace.")
    }
    ksterm.SetJoinKeys($5)
    */
    var ksterm = $4;
    ksterm.join_keys = $5;
    $$ = algebra.NewJoin($1, $2, ksterm)
}
|
from_term opt_join_type JOIN simple_from_term on_key FOR IDENT
{
    /*
    ksterm := algebra.GetKeyspaceTerm($4)
    if ksterm == nil {
        yylex.Error("JOIN must be done on a keyspace.")
    }
    ksterm.SetIndexJoinNest()
    ksterm.SetJoinKeys($5)
    */
    var ksterm = $4;
    ksterm.join_keys = $5;
    $$ = algebra.NewIndexJoin($1, $2, ksterm, $7)
}
|
from_term opt_join_type NEST simple_from_term on_keys
{
    /*
    ksterm := algebra.GetKeyspaceTerm($4)
    if ksterm == nil {
        yylex.Error("NEST must be done on a keyspace.")
    }
    ksterm.SetJoinKeys($5)
    */
    var ksterm = $4;
    ksterm.join_keys = $5;
    $$ = algebra.NewNest($1, $2, ksterm)
}
|
from_term opt_join_type NEST simple_from_term on_key FOR IDENT
{
    /*
    ksterm := algebra.GetKeyspaceTerm($4)
    if ksterm == nil {
        yylex.Error("NEST must be done on a keyspace.")
    }
    ksterm.SetIndexJoinNest()
    ksterm.SetJoinKeys($5)
    */    
    var ksterm = $4;
    ksterm.join_keys = $5;
    $$ = algebra.NewIndexNest($1, $2, ksterm, $7)
}
|
from_term opt_join_type unnest expr opt_as_alias
{
    $$ = algebra.NewUnnest($1, $2, $4, $5)
}
|
from_term opt_join_type JOIN simple_from_term ON expr
{
    /*$4.SetAnsiJoin()*/
    $$ = algebra.NewAnsiJoin($1, $2, $4, $6)
}
|
from_term opt_join_type NEST simple_from_term ON expr
{
    /*$4.SetAnsiNest()*/
    $$ = algebra.NewAnsiNest($1, $2, $4, $6)
}
|
simple_from_term RIGHT opt_outer JOIN simple_from_term ON expr
{
    /*$1.SetAnsiJoin()*/  
    $$ = algebra.NewAnsiRightJoin($4, $5, $7)
}
;

simple_from_term:
keyspace_term
{
    $$ = $1
}
|
expr opt_as_alias opt_use
{
     var other = $1;
     switch ($1.type) {
         case "Subquery":
              if ($2 == "") {
                   yylex.Error("Subquery in FROM clause must have an alias.");
              }
              if ($3 != algebra.EMPTY_USE) {
                   yylex.Error("FROM Subquery cannot have USE KEYS or USE INDEX.");
              }
              $$ = algebra.NewSubqueryTerm(other.Select(), $2);
              break;
         case "Identifier":
              var ksterm = algebra.NewKeyspaceTerm("", other.ops.identifier, $2, $3.Keys(), $3.Indexes());
              //$$ = algebra.NewExpressionTerm(other, $2, ksterm);
              $$ = ksterm;
              break;
         default:
              if ($3 != algebra.EMPTY_USE) {
                  yylex.Error("FROM Expression cannot have USE KEYS or USE INDEX.")
              }
              $$ = algebra.NewExpressionTerm(other,$2, nil);
     }
}
;

unnest:
UNNEST
|
FLATTEN
;

keyspace_term:
keyspace_path opt_as_alias opt_use
{
     var ksterm = algebra.NewKeyspaceTermFromPath($1, $2, $3.Keys(), $3.Indexes());
     $$ = ksterm
}
;

keyspace_path:
namespace_term keyspace_name
{
    $$ = algebra.NewPathShort($1,$2)
}
|
namespace_term bucket_name scope_name DOT keyspace_name
{
    $$ = algebra.NewPathLong($1,$2,$4,$6)
}
;


namespace_term:
namespace_name
|
SYSTEM COLON
{
    $$ = "#system"
}
;

namespace_name:
IDENT COLON 
{
    $$ = $1;
}
;

bucket_name:
IDENT DOT
{
    $$ = $1;
}
;

scope_name:
IDENT
;

keyspace_name:
IDENT
;

opt_use:
/* empty */
{
    $$ = algebra.EMPTY_USE
}
|
USE use_options
{
    $$ = $2
}
;

use_options:
use_keys
|
use_index
|
join_hint
|
use_index join_hint
{
    $1.SetJoinHint($2.JoinHint());
    $$ = $1
}
|
join_hint use_index
{
    $1.SetIndexes($2.Indexes());
    $$ = $1
}
|
use_keys join_hint
{
    $1.SetJoinHint($2.JoinHint());
    $$ = $1
}
|
join_hint use_keys
{
    $1.SetKeys($2.Keys());
    $$ = $1
}
;

use_keys:
opt_primary KEYS expr
{
    $$ = algebra.NewUse($3, nil, algebra.JOIN_HINT_NONE)
}
;

use_index:
INDEX LPAREN index_refs RPAREN
{
    $$ = algebra.NewUse(nil, $3, algebra.JOIN_HINT_NONE)
}
;

join_hint:
HASH LPAREN use_hash_option RPAREN
{
    $$ = algebra.NewUse(nil, nil, $3)
}
|
NL
{
    $$ = algebra.NewUse(nil, nil, algebra.USE_NL)
}
;

opt_primary:
/* empty */
{
}
|
PRIMARY
;

index_refs:
index_ref
{
    $$ = [$1]
}
|
index_refs COMMA index_ref
{
    $1.push($3);
    $$ = $1;
}
;

index_ref:
index_name opt_index_using
{
    $$ = algebra.NewIndexRef($1, $2);
}
;

use_hash_option:
BUILD
{
    $$ = algebra.USE_HASH_BUILD
}
|
PROBE
{
    $$ = algebra.USE_HASH_PROBE
}
;

opt_use_del_upd:
opt_use
{
    /*
    if $1.JoinHint() != algebra.JOIN_HINT_NONE {
        yylex.Error("Keyspace reference cannot have join hint (USE HASH or USE NL) in DELETE or UPDATE statement")
    }
    */
    $$ = $1
}
;

opt_join_type:
/* empty */
{
    $$ = false
}
|
INNER
{
    $$ = false
}
|
LEFT opt_outer
{
    $$ = true
}
;

opt_outer:
/* empty */
|
OUTER
;

on_keys:
ON opt_primary KEYS expr
{
    $$ = $4
}
;

on_key:
ON opt_primary KEY expr
{
    $$ = $4
}
;


/*************************************************
 *
 * LET clause
 *
 *************************************************/

opt_let:
/* empty */
{
    $$ = nil
}
|
let
;

let:
LET bindings
{
    $$ = $2
}
;

bindings:
binding
{
    $$ = [$1]
}
|
bindings COMMA binding
{
    $1.push($3);
    $$ = $1;
}
;

binding:
alias EQ expr
{
    $$ = expression.NewSimpleBinding($1, $3)
}
;

/*************************************************
 *
 * WITH clause
 *
 *************************************************/

opt_with:
WITH with_list
{
    $$ = $2
}
;

with_list:
with_term
{
    $$ = [$1]
}
|
with_list COMMA with_term
{
    $1.push($3);
    $$ = $1;
}
;

with_term:

/* we want expressions in parentesheses, but don't want to be
   forced to have subquery expressions in nested parentheses
 */
alias AS paren_expr
{
    $$ = expression.NewSimpleBinding($1, $3)
}
;


/*************************************************
 *
 * WHERE clause
 *
 *************************************************/

opt_where:
/* empty */
{
    $$ = nil
}
|
where
;

where:
WHERE expr
{
    $$ = $2
}
;


/*************************************************
 *
 * GROUP BY clause
 *
 *************************************************/

opt_group:
/* empty */
{
    $$ = nil
}
|
group
;

group:
GROUP BY group_terms opt_letting opt_having
{
    $$ = algebra.NewGroup($3, $4, $5)
}
|
letting
{
    $$ = algebra.NewGroup(nil, $1, nil)
}
;

group_terms:
group_term
{
    $$ = [$1]
}
|
group_terms COMMA group_term
{
    $1.push($3);
    $$ = $1
}
;

group_term:
expr opt_as_alias
{
    $$ = algebra.NewGroupTerm($1, $2);
}
;

opt_letting:
/* empty */
{
    $$ = nil
}
|
letting
;

letting:
LETTING bindings
{
    $$ = $2
}
;

opt_having:
/* empty */
{
    $$ = nil
}
|
having
;

having:
HAVING expr
{
    $$ = $2
}
;


/*************************************************
 *
 * ORDER BY clause
 *
 *************************************************/

opt_order_by:
/* empty */
{
    $$ = nil
}
|
order_by
;

order_by:
ORDER BY sort_terms
{
    $$ = algebra.NewOrder($3)
}
;

sort_terms:
sort_term
{
    $$ = [$1]
}
|
sort_terms COMMA sort_term
{
    $1.push($3);
    $$ = $1;
}
;

sort_term:
expr opt_dir opt_order_nulls
{
    $$ = algebra.NewSortTerm($1, $2, algebra.NewOrderNullsPos($2,$3));
}
;

opt_dir:
/* empty */
{
    $$ = false
}
|
dir
;

dir:
ASC
{
    $$ = false
}
|
DESC
{
    $$ = true
}
;

opt_order_nulls:
/* empty */
{
    $$ = algebra.NewOrderNulls(true,false,false)
}
|
nulls first_last
{
    $$ = algebra.NewOrderNulls(false, $1,$2)
}
;

first_last:
FIRST { $$ = false }
|
LAST { $$ = true }
;

nulls:
NULLS { $$ = true }
;

/*************************************************
 *
 * LIMIT clause
 *
 *************************************************/

opt_limit:
/* empty */
{
    $$ = nil
}
|
limit
;

limit:
LIMIT expr
{
    $$ = $2
}
;


/*************************************************
 *
 * OFFSET clause
 *
 *************************************************/

opt_offset:
/* empty */
{
    $$ = nil
}
|
offset
;

offset:
OFFSET expr
{
    $$ = $2
}
;


/*************************************************
 *
 * INSERT
 *
 *************************************************/

insert:
INSERT INTO keyspace_ref opt_values_header values_list opt_returning
{
    $$ = algebra.NewInsertValues($3, $5, $6)
}
|
INSERT INTO keyspace_ref LPAREN key_expr opt_value_expr RPAREN fullselect opt_returning
{
    $$ = algebra.NewInsertSelect($3, $5, $6, $8, $9)
}
;

keyspace_ref:
namespace_term keyspace_name opt_as_alias
{
    $$ = algebra.NewKeyspaceRef($1, $2, $3)
}
|
keyspace_name opt_as_alias
{
    $$ = algebra.NewKeyspaceRef("", $1, $2)
}
;

opt_values_header:
/* empty */
|
LPAREN KEY COMMA VALUE RPAREN
|
LPAREN PRIMARY KEY COMMA VALUE RPAREN
;

key:
KEY
|
PRIMARY KEY
;

values_list:
values
|
values_list COMMA next_values
{
    $1.push($3);
    $$ = $1;
}
;

values:
VALUES LPAREN expr COMMA expr RPAREN
{
    $$ = [{Key: $3, Value: $5}];
}
;

next_values:
values {$$ = $1;}
|
LPAREN expr COMMA expr RPAREN
{
    $$ = [{Key: $2, Value: $4}];
}
;

opt_returning:
/* empty */
{
    $$ = nil
}
|
returning
;

returning:
RETURNING returns
{
    $$ = $2
}
;

returns:
projects
{
    $$ = algebra.NewProjection(false, $1)
}
|
raw expr
{
    $$ = algebra.NewRawProjection(false, $2, "")
}
;

key_expr:
key expr
{
    $$ = $2
}
;

opt_value_expr:
/* empty */
{
    $$ = nil
}
|
value_expr
{
    $$ = $1
}
;

value_expr:
COMMA VALUE expr
{
    $$ = $3
}
;


/*************************************************
 *
 * UPSERT
 *
 *************************************************/

upsert:
UPSERT INTO keyspace_ref opt_values_header values_list opt_returning
{
    $$ = algebra.NewUpsertValues($3, $5, $6)
}
|
UPSERT INTO keyspace_ref LPAREN key_expr opt_value_expr RPAREN fullselect opt_returning
{
    $$ = algebra.NewUpsertSelect($3, $5, $6, $8, $9)
}
;


/*************************************************
 *
 * DELETE
 *
 *************************************************/

delete:
DELETE FROM keyspace_ref opt_use_del_upd opt_where opt_limit opt_returning
{
    $$ = algebra.NewDelete($3, $4.Keys(), $4.Indexes(), $5, $6, $7)
}
;


/*************************************************
 *
 * UPDATE
 *
 *************************************************/

update:
UPDATE keyspace_ref opt_use_del_upd set unset opt_where opt_limit opt_returning
{
    $$ = algebra.NewUpdate($2, $3.Keys(), $3.Indexes(), $4, $5, $6, $7, $8)
}
|
UPDATE keyspace_ref opt_use_del_upd set opt_where opt_limit opt_returning
{
    $$ = algebra.NewUpdate($2, $3.Keys(), $3.Indexes(), $4, nil, $5, $6, $7)
}
|
UPDATE keyspace_ref opt_use_del_upd unset opt_where opt_limit opt_returning
{
    $$ = algebra.NewUpdate($2, $3.Keys(), $3.Indexes(), nil, $4, $5, $6, $7)
}
;

set:
SET set_terms
{
    $$ = algebra.NewSet($2)
}
;

set_terms:
set_term
{
    $$ = [$1];
}
|
set_terms COMMA set_term
{
    $1.push($3);
    $$ = $1;
}
;

set_term:
path EQ expr opt_update_for
{
    $$ = algebra.NewSetTerm($1, $3, $4)
}
;

opt_update_for:
/* empty */
{
    $$ = nil
}
|
update_for
;

update_for:
update_dimensions opt_when END
{
    $$ = algebra.NewUpdateFor($1, $2)
}
;

update_dimensions:
FOR update_dimension
{
    $$ = [$2];
}
|
update_dimensions FOR update_dimension
{
    dims = [$3,$1];
}
;

update_dimension:
update_binding
{
    $$ = [$1]
}
|
update_dimension COMMA update_binding
{
    $1.push($3);
    $$ = $1;
}
;

update_binding:
variable IN expr
{
    $$ = expression.NewSimpleBinding($1, $3)
}
|
variable WITHIN expr
{
    $$ = expression.NewBinding("", $1, $3, true)
}
|
variable COLON variable IN expr
{
    $$ = expression.NewBinding($1, $3, $5, false)
}
|
variable COLON variable WITHIN expr
{
    $$ = expression.NewBinding($1, $3, $5, true)
}
;

variable:
IDENT
;

opt_when:
/* empty */
{
    $$ = nil
}
|
WHEN expr
{
    $$ = $2
}
;

unset:
UNSET unset_terms
{
    $$ = algebra.NewUnset($2)
}
;

unset_terms:
unset_term
{
    $$ = [$1]
}
|
unset_terms COMMA unset_term
{
    $1.push($3);
    $$ = $1;
}
;

unset_term:
path opt_update_for
{
    $$ = algebra.NewUnsetTerm($1, $2)
}
;


/*************************************************
 *
 * MERGE
 *
 *************************************************/

merge:
MERGE INTO keyspace_ref opt_use_merge USING simple_from_term ON opt_key expr merge_actions opt_limit opt_returning
{
     switch ($6.type) {
         case "SubqueryTerm":
              var source = algebra.NewMergeSourceSelect($6.Subquery(), $6.Alias())
              $$ = algebra.NewMerge($3, $4.Indexes(), source, $8, $9, $10, $11, $12)
              break;
         case "ExpressionTerm":
              var source = algebra.NewMergeSourceExpression($6, "")
              $$ = algebra.NewMerge($3, $4.Indexes(), source, $8, $9, $10, $11, $12)
              break;
         case "KeyspaceTerm":
              var source = algebra.NewMergeSourceFrom($6, "")
              $$ = algebra.NewMerge($3, $4.Indexes(), source, $8, $9, $10, $11, $12)
              break;
         default:
              yylex.Error("MERGE source term is UNKNOWN: " + $6.type);

     }
}
;

opt_use_merge:
opt_use
{
    /*
    if $1.Keys() != nil {
        yylex.Error("Keyspace reference cannot have USE KEYS hint in MERGE statement.")
    } else if $1.JoinHint() != algebra.JOIN_HINT_NONE {
        yylex.Error("Keyspace reference cannot have join hint (USE HASH or USE NL)in MERGE statement.")
    }
    */
    $$ = $1
}
;

opt_key:
/* empty */
{
    $$ = false
}
|
key
{
    $$ = true
}
;

merge_actions:
/* empty */
{
    $$ = algebra.NewMergeActions(nil, nil, nil)
}
|
WHEN MATCHED THEN UPDATE merge_update opt_merge_delete_insert
{
    $$ = algebra.NewMergeActions($5, $6.Delete(), $6.Insert())
}
|
WHEN MATCHED THEN DELETE merge_delete opt_merge_insert
{
    $$ = algebra.NewMergeActions(nil, $5, $6)
}
|
WHEN NOT MATCHED THEN INSERT merge_insert
{
    $$ = algebra.NewMergeActions(nil, nil, $6)
}
;

opt_merge_delete_insert:
/* empty */
{
    $$ = algebra.NewMergeActions(nil, nil, nil)
}
|
WHEN MATCHED THEN DELETE merge_delete opt_merge_insert
{
    $$ = algebra.NewMergeActions(nil, $5, $6)
}
|
WHEN NOT MATCHED THEN INSERT merge_insert
{
    $$ = algebra.NewMergeActions(nil, nil, $6)
}
;

opt_merge_insert:
/* empty */
{
    $$ = nil
}
|
WHEN NOT MATCHED THEN INSERT merge_insert
{
    $$ = $6
}
;

merge_update:
set opt_where
{
    $$ = algebra.NewMergeUpdate($1, nil, $2)
}
|
set unset opt_where
{
    $$ = algebra.NewMergeUpdate($1, $2, $3)
}
|
unset opt_where
{
    $$ = algebra.NewMergeUpdate(nil, $1, $2)
}
;

merge_delete:
opt_where
{
    $$ = algebra.NewMergeDelete($1)
}
;

merge_insert:
expr opt_where
{
    $$ = algebra.NewMergeInsert(nil,$1,$2)
}
|
LPAREN expr COMMA expr RPAREN opt_where
{
    $$ = algebra.NewMergeInsert($2, $4, $6)
}
|
LPAREN key_expr value_expr RPAREN opt_where
{
    $$ = algebra.NewMergeInsert($2, $3, $5)
}
;

/*************************************************
 *
 * GRANT ROLE
 *
 *************************************************/

grant_role:
GRANT role_list TO user_list
{
    $$ = algebra.NewGrantRole($2, nil, $4)
}
|
GRANT role_list ON keyspace_list TO user_list
{
    $$ = algebra.NewGrantRole($2, $4, $6)
}
;

role_list:
role_name
{
        $$ = [$1];
}
|
role_list COMMA role_name
{
        $1.push($3);
        $$ = $1;
}
;

role_name:
IDENT
{
    $$ = $1
}
|
SELECT
{
    $$ = "select"
}
|
INSERT
{
    $$ = "insert"
}
|
UPDATE
{
    $$ = "update"
}
|
DELETE
{
    $$ = "delete"
}
;

keyspace_list:
IDENT
{
    $$ = [$1];
}
|
keyspace_list COMMA IDENT
{
    $1.push($3);
    $$ = $1;
}
;

user_list:
user
{
    $$ = [$1]
}
|
user_list COMMA user
{
    $1.push($3);
    $$ = $1;
}
;

user:
IDENT
{
    $$ = $1;
}
|
IDENT COLON IDENT
{
    $$ = $1 + ":" + $3;
}
;

/*************************************************
 *
 * REVOKE ROLE
 *
 *************************************************/

revoke_role:
REVOKE role_list FROM user_list
{
    $$ = algebra.NewRevokeRole($2, nil, $4);
}
|
REVOKE role_list ON keyspace_list FROM user_list
{
    $$ = algebra.NewRevokeRole($2, $4, $6);
}
;

/*************************************************
 *
 * CREATE INDEX
 *
 *************************************************/

create_index:
CREATE PRIMARY INDEX opt_primary_name ON named_keyspace_ref index_partition opt_index_using opt_index_with
{
    $$ = algebra.NewCreatePrimaryIndex($4, $6, $7, $8, $9)
}
|
CREATE INDEX index_name ON named_keyspace_ref LPAREN index_terms RPAREN index_partition index_where opt_index_using opt_index_with
{
    $$ = algebra.NewCreateIndex($3, $5, $7, $9, $10, $11, $12)
}
;

opt_primary_name:
/* empty */
{
    $$ = "#primary"
}
|
index_name
;

index_name:
IDENT
;

named_keyspace_ref:
keyspace_name
{
    $$ = algebra.NewKeyspaceRef("", $1, "")
}
|
namespace_name keyspace_name
{
    $$ = algebra.NewKeyspaceRef($1, $2, "")
}
;

index_partition:
/* empty */
{
    $$ = nil
}
|
PARTITION BY HASH LPAREN exprs RPAREN
{
    $$ = $5
}
;

opt_index_using:
/* empty */
{
    $$ = datastore.DEFAULT
}
|
index_using
;

index_using:
USING VIEW
{
    $$ = datastore.VIEW
}
|
USING GSI
{
    $$ = datastore.GSI
}
|
USING FTS
{
    $$ = datastore.FTS
}
;

opt_index_with:
/* empty */
{
    $$ = nil
}
|
index_with
;

index_with:
WITH expr
{
    $$ = $2.Value()
    if ($$ == nil) {
        yylex.Error("WITH value must be static.")
    }
}
;

index_terms:
index_term
{
    $$ = [$1]
}
|
index_terms COMMA index_term
{
    $1.push($3);
    $$ = $1;
}
;

index_term:
index_term_expr opt_dir
{
   $$ = algebra.NewIndexKeyTerm($1, $2)
}
;

index_term_expr:
index_expr
|
all index_expr
{
    $$ = expression.NewAll($2, false)
}
|
all DISTINCT index_expr
{
    $$ = expression.NewAll($3, true)
}
|
DISTINCT index_expr
{
    $$ = expression.NewAll($2, true)
}
;

index_expr:
expr
{
    var exp = $1
    //if (exp != nil && (!exp.Indexable() || exp.Value() != nil)) {
    //    yylex.Error(fmt.Sprintf("Expression not indexable: %s", exp.String()))
    //}

    $$ = exp
}
;

all:
ALL
|
EACH
;

index_where:
/* empty */
{
    $$ = nil
}
|
WHERE index_expr
{
    $$ = $2
}
;


/*************************************************
 *
 * DROP INDEX
 *
 *************************************************/

drop_index:
DROP PRIMARY INDEX ON named_keyspace_ref opt_index_using
{
    $$ = algebra.NewDropIndex($5, "#primary", $6) 
}
|
DROP INDEX named_keyspace_ref DOT index_name opt_index_using
{
    $$ = algebra.NewDropIndex($3, $5, $6)
}
;

/*************************************************
 *
 * ALTER INDEX
 *
 *************************************************/

alter_index:
ALTER INDEX named_keyspace_ref DOT index_name opt_index_using index_with
{
    $$ = algebra.NewAlterIndex($3, $5, $6, $7)
}
;

/*************************************************
 *
 * BUILD INDEX
 *
 *************************************************/

build_index:
BUILD INDEX ON named_keyspace_ref LPAREN exprs RPAREN opt_index_using
{
    $$ = algebra.NewBuildIndexes($4, $8, $6)
}
;

/*************************************************
 *
 * CREATE FUNCTION
 *
 *************************************************/

create_function:
CREATE FUNCTION func_name LPAREN parm_list RPAREN func_body
{
    /*
    if $7 != nil {
    err := $7.SetVarNames($5)
    if err != nil {
        yylex.Error(err.Error())
        }
    }
    */
    $$ = algebra.NewCreateFunction($3, $7, $5);
}
;

func_name:
short_func_name
|
long_func_name
;

short_func_name:
keyspace_name
{
    /*
    name, err := functions.Constructor([]string{$1}, yylex.(*lexer).Namespace())
    if err != nil {
    yylex.Error(err.Error())
    }
    $$ = name
    */
    $$ = $1;
}
;

long_func_name:
namespace_term keyspace_name
{
    /*
    name, err := functions.Constructor([]string{$1, $2}, yylex.(*lexer).Namespace())
    if $$ != nil {
    yylex.Error(err.Error())
    }
    $$ = name
    */
    $$ = [$1,$2];
}
/* TODO function names for collections
|
namespace_term bucket_name scope_name DOT keyspace_name
{
    name, err := functions.Constructor([]string{$1, $2, $4, $6}, yylex.(*lexer).Namespace())
    if $$ != nil {
    yylex.Error(err.Error())
    }
    $$ = name
    //$$ = [$1,$2,$4,$6];
}
*/
;

parm_list:
/* empty */
{
    $$ = nil
}
|
parameter_terms
;

parameter_terms:
IDENT
{
    $$ = [$1]
}
|
parameter_terms COMMA IDENT
{
    $1.push($3);
    $$ = $1;
}
;

func_body:
LBRACE expr RBRACE
{
    $$ = $2;
    /*
    body, err := inline.NewInlineBody($2)
    if err != nil {
    yylex.Error(err.Error())
    } else {
        $$ = body
    }
    */
}
|
LANGUAGE INLINE AS expr
{
    $$ = $4;
    /*
    body, err := inline.NewInlineBody($4)
    if err != nil {
    yylex.Error(err.Error())
    } else {
        $$ = body
    }
    */
}
|
LANGUAGE GOLANG AS LBRACE STR COMMA STR RBRACE
{   
    $$ = [$5,$7]
    /*
    body, err := golang.NewGolangBody($5, $7)
    if err != nil {
        yylex.Error(err.Error())
    } else { 
        $$ = body
    }
    */
}
|
LANGUAGE JAVASCRIPT AS LBRACE STR COMMA STR RBRACE
{
   $$ = [$5,$7]
   /*
    body, err := javascript.NewJavascriptBody($5, $7)
    if err != nil {
        yylex.Error(err.Error())
    } else {
        $$ = body
    } 
   */
}
;

/*************************************************
 *
 * DROP FUNCTION
 *
 *************************************************/

drop_function:
DROP FUNCTION func_name
{
    $$ = algebra.NewDropFunction($3)
}
;

/*************************************************
 *
 * EXECUTE FUNCTION
 *
 *************************************************/

execute_function:
EXECUTE FUNCTION func_name LPAREN opt_exprs RPAREN
{
    $$ = algebra.NewExecuteFunction($3, $5)
}
;


/*************************************************
 *
 * UPDATE STATISTICS
 *
 *************************************************/

update_statistics:
UPDATE STATISTICS opt_for named_keyspace_ref LPAREN update_stat_terms RPAREN opt_infer_ustat_with
{
    $$ = algebra.NewUpdateStatistics($4, $6, $8)
}
;

opt_for:
/* empty */
|
FOR
;

update_stat_terms:
update_stat_term
{
    $$ = [$1]
}
|
update_stat_terms COMMA update_stat_term
{
    $1.push($3);
    $$ = $1;
}
;

update_stat_term:
index_term_expr
;

/*************************************************
 *
 * Path
 *
 *************************************************/

path:
IDENT
{
    $$ = expression.NewIdentifier($1)
}
|
path DOT IDENT
{
    $$ = expression.NewField($1, expression.NewFieldName($3, false));
}
|
path DOT IDENT_ICASE
{
    var field = expression.NewField($1, expression.NewFieldName($3, true))
    field.SetCaseInsensitive = true;
    $$ = field
}
|
path DOT LBRACKET expr RBRACKET
{
    $$ = expression.NewField($1, $4)
}
|
path DOT LBRACKET expr RBRACKET_ICASE
{
    var field = expression.NewField($1, $4)
    field.SetCaseInsensitive = true;
    $$ = field
}
|
path LBRACKET expr RBRACKET
{
    $$ = expression.NewElement($1, $3)
}
;


/*************************************************
 *
 * Expression
 *
 *************************************************/

expr:
c_expr
|
/* Nested */
expr DOT IDENT
{
    $$ = expression.NewField($1, expression.NewFieldName($3, false))
}
|
expr DOT IDENT_ICASE
{
    var field = expression.NewField($1, expression.NewFieldName($3, true))
    field.SetCaseInsensitive = true;
    $$ = field
}
|
expr DOT LBRACKET expr RBRACKET
{
    $$ = expression.NewField($1, $4)
}
|
expr DOT LBRACKET expr RBRACKET_ICASE
{
    var field = expression.NewField($1, $4)
    field.SetCaseInsensitive = true;
    $$ = field
}
|
expr LBRACKET expr RBRACKET
{
    $$ = expression.NewElement($1, $3)
}
|
expr LBRACKET expr COLON RBRACKET
{
    $$ = expression.NewSlice($1, $3)
}
|
expr LBRACKET expr COLON expr RBRACKET
{
    $$ = expression.NewSlice($1, $3, $5)
}
|
expr LBRACKET STAR RBRACKET
{
    $$ = expression.NewArrayStar($1)
}
|
/* Arithmetic */
expr PLUS expr
{
    $$ = expression.NewAdd($1, $3)
}
|
expr MINUS expr
{
    $$ = expression.NewSub($1, $3)
}
|
expr STAR expr
{
    $$ = expression.NewMult($1, $3)
}
|
expr DIV expr
{
    $$ = expression.NewDiv($1, $3)
}
|
expr MOD expr
{
    $$ = expression.NewMod($1, $3)
}
|
/* Concat */
expr CONCAT expr
{
    $$ = expression.NewConcat($1, $3)
}
|
/* Logical */
expr AND expr
{
    $$ = expression.NewAnd($1, $3)
}
|
expr OR expr
{
    $$ = expression.NewOr($1, $3)
}
|
NOT expr
{
    $$ = expression.NewNot($2)
}
|
/* Comparison */
expr EQ expr
{
    $$ = expression.NewEq($1, $3)
}
|
expr DEQ expr
{
    $$ = expression.NewEq($1, $3)
}
|
expr NE expr
{
    $$ = expression.NewNE($1, $3)
}
|
expr LT expr
{
    $$ = expression.NewLT($1, $3)
}
|
expr GT expr
{
    $$ = expression.NewGT($1, $3)
}
|
expr LE expr
{
    $$ = expression.NewLE($1, $3)
}
|
expr GE expr
{
    $$ = expression.NewGE($1, $3)
}
|
expr BETWEEN b_expr AND b_expr
{
    $$ = expression.NewBetween($1, $3, $5)
}
|
expr NOT BETWEEN b_expr AND b_expr
{
    $$ = expression.NewNotBetween($1, $4, $6)
}
|
expr LIKE expr
{
    $$ = expression.NewLike($1, $3)
}
|
expr NOT LIKE expr
{
    $$ = expression.NewNotLike($1, $4)
}
|
expr IN expr
{
    $$ = expression.NewIn($1, $3)
}
|
expr NOT IN expr
{
    $$ = expression.NewNotIn($1, $4)
}
|
expr WITHIN expr
{
    $$ = expression.NewWithin($1, $3)
}
|
expr NOT WITHIN expr
{
    $$ = expression.NewNotWithin($1, $4)
}
|
expr IS NULL
{
    $$ = expression.NewIsNull($1)
}
|
expr IS NOT NULL
{
    $$ = expression.NewIsNotNull($1)
}
|
expr IS MISSING
{
    $$ = expression.NewIsMissing($1)
}
|
expr IS NOT MISSING
{
    $$ = expression.NewIsNotMissing($1)
}
|
expr IS valued
{
    $$ = expression.NewIsValued($1)
}
|
expr IS NOT valued
{
    $$ = expression.NewIsNotValued($1)
}
|
EXISTS expr
{
    $$ = expression.NewExists($2)
}
;

valued:
VALUED
|
KNOWN
;

c_expr:
/* Literal */
literal
|
/* Construction */
construction_expr
|
/* Identifier */
IDENT
{
    $$ = expression.NewIdentifier($1)
}
|
/* Identifier */
IDENT_ICASE
{
    var ident = expression.NewIdentifier($1)
    ident.SetCaseInsensitive = true;
    $$ = ident
}
|
/* Self */
SELF
{
    $$ = expression.NewSelf()
}
|
/* Parameter */
param_expr
|
/* Function */
function_expr
|
/* Prefix */
MINUS expr %prec UMINUS
{
    $$ = expression.NewNeg($2)
}
|
/* Case */
case_expr
|
/* Collection */
collection_expr
|
/* Grouping and subquery */
paren_expr
|
/* For covering indexes */
COVER LPAREN expr RPAREN
{
    $$ = expression.NewCover($3)
}
;

b_expr:
c_expr
|
/* Nested */
b_expr DOT IDENT
{
    $$ = expression.NewField($1, expression.NewFieldName($3, false));
}
|
b_expr DOT IDENT_ICASE
{
    var field = expression.NewField($1, expression.NewFieldName($3, true))
    field.SetCaseInsensitive = true;
    $$ = field
}
|
b_expr DOT LBRACKET expr RBRACKET
{
    $$ = expression.NewField($1, $4)
}
|
b_expr DOT LBRACKET expr RBRACKET_ICASE
{
    var field = expression.NewField($1, $4)
    field.SetCaseInsensitive = true;
    $$ = field
}
|
b_expr LBRACKET expr RBRACKET
{
    $$ = expression.NewElement($1, $3)
}
|
b_expr LBRACKET expr COLON RBRACKET
{
    $$ = expression.NewSlice($1, $3)
}
|
b_expr LBRACKET expr COLON expr RBRACKET
{
    $$ = expression.NewSlice($1, $3, $5)
}
|
b_expr LBRACKET STAR RBRACKET
{
    $$ = expression.NewArrayStar($1)
}
|
/* Arithmetic */
b_expr PLUS b_expr
{
    $$ = expression.NewAdd($1, $3)
}
|
b_expr MINUS b_expr
{
    $$ = expression.NewSub($1, $3)
}
|
b_expr STAR b_expr
{
    $$ = expression.NewMult($1, $3)
}
|
b_expr DIV b_expr
{
    $$ = expression.NewDiv($1, $3)
}
|
b_expr MOD b_expr
{
    $$ = expression.NewMod($1, $3)
}
|
/* Concat */
b_expr CONCAT b_expr
{
    $$ = expression.NewConcat($1, $3)
}
;


/*************************************************
 *
 * Literal
 *
 *************************************************/

literal:
NULL
{
    $$ = expression.NULL_EXPR
}
|
MISSING
{
    $$ = expression.MISSING_EXPR
}
|
FALSE
{
    $$ = expression.FALSE_EXPR
}
|
TRUE
{
    $$ = expression.TRUE_EXPR
}
|
NUM
{
    $$ = expression.NewConstant(value.NewValue($1))
}
|
INT
{
    $$ = expression.NewConstant(value.NewValue($1))
}
|
STR
{
    $$ = expression.NewConstant(value.NewValue($1))
}
;


/*************************************************
 *
 * Construction
 *
 *************************************************/

construction_expr:
object
|
array
;

object:
LBRACE opt_members RBRACE
{
    $$ = expression.NewObjectConstruct(algebra.MapPairs($2))
}
;

opt_members:
/* empty */
{
    $$ = nil
}
|
members
;

members:
member
{
    $$ = [$1]
}
|
members COMMA member
{
    $1.push($3);
    $$ = $1;
}
;

member:
expr COLON expr
{
    $$ = algebra.NewPair($1, $3)
}
|
expr
{
    var name = $1.Alias()
    if (name == "") {
        yylex.Error(fmt.Sprintf("Object member missing name or value: %s", $1.String()))
    }

    $$ = algebra.NewPair(expression.NewConstant(name), $1)
}
;

array:
LBRACKET opt_exprs RBRACKET
{
    $$ = expression.NewArrayConstruct($2)
}
;

opt_exprs:
/* empty */
{
    $$ = nil
}
|
exprs
;

exprs:
expr
{
    $$ = [$1]
}
|
exprs COMMA expr
{
    $1.push($3);
    $$ = $1;
}
;

/*************************************************
 *
 * Parameter
 *
 *************************************************/

param_expr:
NAMED_PARAM
{
    $$ = algebra.NewNamedParameter($1);
}
|
POSITIONAL_PARAM
{
    $$ = algebra.NewPositionalParameter($1);
}
|
NEXT_PARAM
{
    $$ = algebra.NewPositionalParameter($1);
}
;


/*************************************************
 *
 * Case
 *
 *************************************************/

case_expr:
CASE simple_or_searched_case END
{
    $$ = $2
}
;

simple_or_searched_case:
simple_case
|
searched_case
;

simple_case:
expr when_thens opt_else
{
    $$ = expression.NewSimpleCase($1, $2, $3)
}
;

when_thens:
WHEN expr THEN expr
{
    $$ = [{when: $2, then: $4}]
}
|
when_thens WHEN expr THEN expr
{
    $1.push({when: $3, then: $5});
    $$ = $1;
}
;

searched_case:
when_thens
opt_else
{
    $$ = expression.NewSearchedCase($1, $2)
}
;

opt_else:
/* empty */
{
    $$ = nil
}
|
ELSE expr
{
    $$ = $2
}
;


/*************************************************
 *
 * Function
 *
 * NTH_VALUE(expr,n) [FROM FIRST|LAST] [RESPECT|IGNORE NULLS] OVER(....)
 *   requires special handling due to FROM (avoid conflict with query FROM)
 *   example: SELECT SUM(c1) FROM default WHERE ...
 *************************************************/

function_expr:
NTH_VALUE LPAREN exprs RPAREN opt_from_first_last opt_nulls_treatment window_clause
{
    var fname = "nth_value";
    $$ = algebra.GetAggregate(fname, false, ($7 != null));
}
|
function_name opt_exprs RPAREN opt_nulls_treatment opt_window_clause
{
    $$ = expression.NewFunction($1,$2);
}
|
function_name agg_quantifier expr RPAREN opt_window_clause
{
    $$ = expression.NewFunction($1,$3,true);
}
|
function_name STAR RPAREN opt_window_clause
{
    $$ = expression.NewFunction($1,"star");
}
|
namespace_term keyspace_name LPAREN opt_exprs RPAREN
{
    $$ = expression.NewFunction($2,$4);
}
;

function_name:
IDENT LPAREN {$$ = $1;}
;


/*************************************************
 *
 * Collection
 *
 *************************************************/

collection_expr:
collection_cond
|
collection_xform
;

collection_cond:
ANY coll_bindings satisfies END
{
    $$ = expression.NewAny($2, $3)
}
|
SOME coll_bindings satisfies END
{
    $$ = expression.NewAny($2, $3)
}
|
EVERY coll_bindings satisfies END
{
    $$ = expression.NewEvery($2, $3)
}
|
ANY AND EVERY coll_bindings satisfies END
{
    $$ = expression.NewAnyEvery($4, $5)
}
|
SOME AND EVERY coll_bindings satisfies END
{
    $$ = expression.NewAnyEvery($4, $5)
}
;

coll_bindings:
coll_binding
{
    $$ = [$1];
}
|
coll_bindings COMMA coll_binding
{
    $1.push($3);
    $$ = $1;
}
;

coll_binding:
variable IN expr
{
    $$ = expression.NewSimpleBinding($1, $3)
}
|
variable WITHIN expr
{
    $$ = expression.NewBinding("", $1, $3, true)
}
|
variable COLON variable IN expr
{
    $$ = expression.NewBinding($1, $3, $5, false)
}
|
variable COLON variable WITHIN expr
{
    $$ = expression.NewBinding($1, $3, $5, true)
}
;

satisfies:
SATISFIES expr
{
    $$ = $2
}
;

collection_xform:
ARRAY expr FOR coll_bindings opt_when END
{
    $$ = expression.NewArray($2, $4, $5)
}
|
FIRST expr FOR coll_bindings opt_when END
{
    $$ = expression.NewFirst($2, $4, $5)
}
|
OBJECT expr COLON expr FOR coll_bindings opt_when END
{
    $$ = expression.NewObject($2, $4, $6, $7)
}
;


/*************************************************
 *
 * Parentheses and subquery
 *
 *************************************************/

paren_expr:
LPAREN expr RPAREN
{
    $$ = $2
}
|
LPAREN all_expr RPAREN
{
    $$ = $2
}
|
subquery_expr
{
    $$ = $1
}
;

subquery_expr:
CORRELATED LPAREN fullselect RPAREN
{
    $$ = algebra.NewSubquery($2);
}
|
LPAREN fullselect RPAREN
{
    $$ = algebra.NewSubquery($2);
}
;


/*************************************************
 *
 * Top-level expression input / parsing.
 *
 *************************************************/

expr_input:
expr
|
all_expr
;

all_expr:
all expr
{
    $$ = expression.NewAll($2, false)
}
|
all DISTINCT expr
{
    $$ = expression.NewAll($3, true)
}
|
DISTINCT expr
{
    $$ = expression.NewAll($2, true)
}
;

opt_window_clause:
/* empty */
{ $$ = nil }
|
window_clause
{ $$ = $1 }
;

window_clause:
OVER LPAREN opt_window_partition opt_order_by opt_window_frame RPAREN
{
    $$ = algebra.NewWindowTerm($3,$4,$5)
}
;

opt_window_partition:
/* empty */
{ $$ = nil }
|
PARTITION BY exprs
{ $$ = $3 }
;

opt_window_frame:
/* empty */
{
    $$ = nil
}
|
window_frame_modifier window_frame_extents opt_window_frame_exclusion
{
    $$ = algebra.NewWindowFrame($1|$3, $2)
}
;


window_frame_modifier:
ROWS
{
    $$ = algebra.WINDOW_FRAME_ROWS
}
|
RANGE
{
    $$ = algebra.WINDOW_FRAME_RANGE
}
|
GROUPS
{
    $$ = algebra.WINDOW_FRAME_GROUPS
}
;

opt_window_frame_exclusion:
/* empty */
{
     $$ = 0
}
|
EXCLUDE NO OTHERS
{
     $$ = 0
}
|
EXCLUDE CURRENT ROW
{
     $$ = algebra.WINDOW_FRAME_EXCLUDE_CURRENT_ROW
}
|
EXCLUDE TIES
{
     $$ = algebra.WINDOW_FRAME_EXCLUDE_TIES
}
|
EXCLUDE GROUP
{
     $$ = algebra.WINDOW_FRAME_EXCLUDE_GROUP
}
;

window_frame_extents:
window_frame_extent
{
    $$ = algebra.WindowFrameExtents($1)
}
|
BETWEEN window_frame_extent AND window_frame_extent
{
    $$ = algebra.WindowFrameExtents($2, $4)
}
;

window_frame_extent:
UNBOUNDED PRECEDING
{
    $$ = algebra.NewWindowFrameExtent(nil, algebra.WINDOW_FRAME_UNBOUNDED_PRECEDING)
}
|
UNBOUNDED FOLLOWING
{
    $$ = algebra.NewWindowFrameExtent(nil, algebra.WINDOW_FRAME_UNBOUNDED_FOLLOWING)
}
|
CURRENT ROW
{
    $$ = algebra.NewWindowFrameExtent(nil, algebra.WINDOW_FRAME_CURRENT_ROW)
}
|
expr window_frame_valexpr_modifier
{
    $$ = algebra.NewWindowFrameExtent($1, $2)
}
;

window_frame_valexpr_modifier:
PRECEDING
{
    $$ = algebra.WINDOW_FRAME_VALUE_PRECEDING
}
|
FOLLOWING
{
    $$ = algebra.WINDOW_FRAME_VALUE_FOLLOWING
}
;

opt_nulls_treatment:
/* empty */
{ $$ = 0 }
|
nulls_treatment
{ $$ = $1 }
;

nulls_treatment:
RESPECT NULLS
{ $$ = algebra.AGGREGATE_RESPECTNULLS }
|
IGNORE NULLS
{ $$ = algebra.AGGREGATE_IGNORENULLS }
;

opt_from_first_last:
/* empty */
{ $$ = 0 }
|
FROM first_last
{
    if ($2) {
         $$ = algebra.AGGREGATE_FROMLAST
    } else {
         $$ = algebra.AGGREGATE_FROMFIRST
    }
}
;

agg_quantifier:
ALL
{
   $$ = 0
}
|
DISTINCT
{
   $$ = algebra.AGGREGATE_DISTINCT
}
;