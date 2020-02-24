// n1ql-validator.js

var parser = require("./n1ql").parser;

function queryArray() {
    var queries = [
      "select default:func('hotel') from `travel-sample`;",
      "default:func('hotel')"
      //"Update default set foo = 'bar'",
      //"delete from default",
      //"delete from default where foo = bar",
      //"select count(*) from default; select max(foo) from bar"
      /*
        "MERGE INTO orders USING orders o USE KEYS ['subqexp_1235', 'subqexp_1236'] ON KEY id WHEN NOT MATCHED THEN INSERT {o.id,'test_id':'subqexp'};",
        "MERGE INTO orders USING (SELECT 's'||id  AS id FROM orders WHERE test_id = 'subqexp' ) o ON KEY o.id WHEN NOT MATCHED THEN INSERT {o.id,'test_id':'subqexp'};",
        "MERGE INTO orders USING (SELECT 'se'||id  AS id, (SELECT RAW SUM(orderlines.price) FROM orders.orderlines)[0] AS total FROM orders WHERE test_id = 'subqexp') o ON KEY o.id WHEN NOT MATCHED THEN INSERT {o.id, o.total, 'test_id':'subqexp'};",
        "MERGE INTO orders USING [{'id':'c1235'},{'id':'c1236'}] o ON KEY id WHEN NOT MATCHED THEN INSERT {o.id, 'test_id':'subqexp'};",

"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p ON p.customerId = c.customerId OR p.customerId = \"unknown\" WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p ON p.customerId = c.customerId OR p.purchaseId = \"purchase8992\" WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10 OFFSET 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p ON p.customerId IN [ c.customerId, \"unknown\" ] WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10 OFFSET 20",
"SELECT p.productId, pu.customerId FROM product p JOIN purchase pu ON ANY pd IN pu.lineItems satisfies p.productId = pd.product END WHERE ANY r IN p.reviewList satisfies r = \"review1636\" END ORDER BY pu.customerId LIMIT 5",
"SELECT p.productId, pu.customerId, pu.purchaseId FROM product p JOIN purchase pu ON ANY pd IN pu.lineItems satisfies p.productId = pd.product END WHERE ANY r IN p.reviewList satisfies r = \"review1636\" END ORDER BY pu.customerId LIMIT 5",
"SELECT p.productId, p.color, pu.customerId FROM purchase pu JOIN product p ON p.productId IN ARRAY pd.product FOR pd IN pu.lineItems END WHERE pu.purchaseId = \"purchase1000\" ORDER BY p.productId",
"SELECT p.productId, p.color, pu.customerId FROM purchase pu UNNEST pu.lineItems as pl JOIN product p ON p.productId = pl.product WHERE pu.purchaseId = \"purchase1000\" ORDER BY p.productId",
"SELECT p.productId, pu.customerId FROM purchase pu JOIN product p ON ANY pd IN pu.lineItems satisfies pd.product = p.productId END WHERE pu.purchaseId = \"purchase1000\" ORDER BY p.productId",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM purchase p JOIN customer c ON meta(c).id = p.customerId || \"_\" || p.test_id WHERE p.purchaseId LIKE \"purchase655%\" ORDER BY p.purchaseId",
"SELECT p.productId, pu.customerId, pu.purchaseId FROM purchase pu JOIN product p ON meta(p).id IN ARRAY (pd.product || \"_ansijoin\") FOR pd IN pu.lineItems END WHERE pu.purchaseId = \"purchase1000\" ORDER BY p.productId",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p ON meta(c).id = p.customerId || \"_\" || p.test_id WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" AND c.type = \"customer\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" AND p.type = \"purchase\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" AND c.type = \"customer\" AND p.type = \"purchase\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c LEFT OUTER JOIN purchase p ON c.customerId = p.customerId WHERE c.lastName = \"Wyman\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM purchase p RIGHT OUTER JOIN customer c ON c.customerId = p.customerId WHERE c.lastName = \"Wyman\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.customerId, p.purchaseId FROM customer c JOIN purchase p ON c.customerId = p.customerId ORDER BY p.purchaseId LIMIT 4",
"SELECT c.customerId, p.purchaseId FROM customer c JOIN purchase p ON c.customerId  || \"1\" = p.customerId ORDER BY p.purchaseId LIMIT 4",
"SELECT b1.c11, b2.c21, b2.c22 FROM shellTest b1 JOIN shellTest b2 ON b1.a11 = b2.a22 WHERE b1.type = \"left\" AND b2.type = \"right\" ORDER BY b2.c22",
"SELECT b2.c21, b2.c22, b2.a21 FROM shellTest b1 JOIN shellTest b2 ON b1.c11 = b2.c21 AND ANY v IN b2.a21 SATISFIES v = 10 END AND b2.type = \"right\" WHERE b1.type = \"left\" ORDER BY b2.c22",
"SELECT b1.c11, b2.c21, b2.c22 FROM shellTest b1 JOIN shellTest b2 ON b2.c21 = b1.c11 AND ANY v IN b2.a21 SATISFIES v = b1.c12 END AND b2.type = \"right\" WHERE b1.type = \"left\" AND b1.c11 IS NOT MISSING ORDER BY b2.c22",
"SELECT b1.c11, b2.c21, b2.c22 FROM shellTest b1 UNNEST b1.a11 AS ba1 JOIN shellTest b2 ON ba1 = b2.c21 AND b2.type = \"right\" WHERE b1.c11 = 2 AND b1.type = \"left\" ORDER BY b2.c22",
"SELECT b1.c11, b2.c21, b2.c22 FROM shellTest b1 UNNEST b1.a11 AS ba1 LEFT JOIN shellTest b2 ON ba1 = b2.c21 AND b2.type = \"right\" WHERE b1.c11 = 4 AND b1.type = \"left\"",
"SELECT b1.c11, b2.c21, b2.c22 FROM shellTest b1 JOIN shellTest b2 ON b2.c21 IN b1.a11 AND b2.type = \"right\" WHERE b1.c11 = 2 AND b1.type = \"left\" ORDER BY b2.c22",
"SELECT b1.c11, b2.c21, b2.c22 FROM shellTest b1 LEFT JOIN shellTest b2 ON b2.c21 IN b1.a11 AND b2.type = \"right\" WHERE b1.c11 = 4 AND b1.type = \"left\"",
"SELECT b1.c11, b2.c21, b2.c22 FROM shellTest b1 UNNEST b1.a11 AS ba1 JOIN shellTest b2 ON b2.c21 = b1.c11 AND ANY v IN b2.a21 SATISFIES v = ba1 END AND b2.type = \"right\" WHERE b1.type = \"left\" AND b1.c11 IS NOT MISSING ORDER BY b2.c22",
"SELECT b1.c11, b2.c21, b2.c22 FROM shellTest b1 JOIN shellTest b2 ON b2.c21 = b1.c11 AND ANY v IN b2.a21 SATISFIES v IN b1.a11 END AND b2.type = \"right\" WHERE b1.type = \"left\" AND b1.c11 IS NOT MISSING ORDER BY b2.c22",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c USE INDEX (cust_lastName_firstName_customerId) JOIN purchase p USE INDEX (purch_customerId_purchaseId) ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p USE INDEX (purch_customerId_purchaseId, purch_purchaseId) ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" AND c.type = \"customer\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p USE INDEX (purch_customerId_purchaseId) ON p.customerId = c.customerId OR p.customerId = \"unknown\" WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p USE INDEX (purch_customerId_purchaseId, purch_purchaseId) ON p.customerId = c.customerId OR p.purchaseId = \"purchase8992\" WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10 OFFSET 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c USE INDEX (cust_lastName_firstName_customerId) JOIN purchase p USE KEYS (select raw meta().id from purchase where customerId in [\"customer33\", \"customer60\", \"customer631\"]) ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10",
"SELECT pc.purchaseId, l.product, pd.name FROM purchase pc UNNEST pc.lineItems as l JOIN product pd ON l.product = pd.productId WHERE pc.purchaseId = \"purchase6558\" ORDER BY l.product",
"SELECT pc.purchaseId, l.product, pd.name, c.lastName, c.firstName FROM purchase pc JOIN customer c ON pc.customerId = c.customerId UNNEST pc.lineItems as l JOIN product pd ON l.product = pd.productId WHERE pc.purchaseId = \"purchase6558\" ORDER BY l.product",
"SELECT b1.c11, b2.c21, b2.c22 FROM shellTest b1 JOIN shellTest b2 ON b1.c11 = b2.c21 AND b1.c12 = b2.c22 AND b1.c11 < 3 AND b2.type = \"right\" WHERE b1.type = \"left\" ORDER BY b2.c22",
"SELECT b1.c11, b2.c21, b2.c22 FROM shellTest b1 LEFT JOIN shellTest b2 ON b1.c11 = b2.c21 AND b1.c12 = b2.c22 AND b1.c11 < 3 AND b2.type = \"right\" WHERE b1.type = \"left\" ORDER BY b2.c22",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c USE INDEX (cust_lastName_firstName_customerId) JOIN purchase p USE INDEX (purch_customerId_purchaseId) HASH(probe) ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c USE HASH(build) JOIN purchase p USE INDEX (purch_customerId_purchaseId, purch_purchaseId) ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" AND c.type = \"customer\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p USE HASH(probe) INDEX (purch_customerId_purchaseId, purch_purchaseId) ON p.customerId = c.customerId OR p.purchaseId = \"purchase8992\" WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10 OFFSET 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c USE INDEX (cust_lastName_firstName_customerId) JOIN purchase p USE HASH(probe) KEYS (select raw meta().id from purchase where customerId in [\"customer33\", \"customer60\", \"customer631\"]) ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p USE KEYS [\"purchase1582_hashjoin\", \"purchase1704_hashjoin\", \"purchase4534_hashjoin\", \"purchase5988_hashjoin\", \"purchase6985_hashjoin\", \"purchase7352_hashjoin\", \"purchase8538_hashjoin\", \"purchase8992_hashjoin\", \"purchase9287_hashjoin\", \"purchase104_hashjoin\", \"purchase1747_hashjoin\", \"purchase3344_hashjoin\", \"purchase3698_hashjoin\", \"purchase4142_hashjoin\", \"purchase4315_hashjoin\", \"purchase436_hashjoin\", \"purchase5193_hashjoin\", \"purchase5889_hashjoin\", \"purchase6084_hashjoin\", \"purchase8349_hashjoin\", \"purchase9300_hashjoin\", \"purchase2838_hashjoin\", \"purchase2872_hashjoin\", \"purchase4627_hashjoin\", \"purchase5610_hashjoin\", \"purchase6530_hashjoin\", \"purchase993_hashjoin\"] HASH(build) ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10",
"SELECT pc.purchaseId, l.product, pd.name FROM purchase pc UNNEST pc.lineItems as l JOIN product pd USE HASH(probe) ON l.product = pd.productId WHERE pc.purchaseId = \"purchase6558\" ORDER BY l.product",
"SELECT pc.purchaseId, l.product, pd.name, c.lastName, c.firstName FROM purchase pc JOIN customer c ON pc.customerId = c.customerId UNNEST pc.lineItems as l JOIN product pd USE HASH(probe) ON l.product = pd.productId WHERE pc.purchaseId = \"purchase6558\" ORDER BY l.product",
"SELECT pc.purchaseId, l.product, pd.name, c.lastName, c.firstName FROM purchase pc JOIN customer c USE HASH(build) ON pc.customerId = c.customerId UNNEST pc.lineItems as l JOIN product pd ON l.product = pd.productId WHERE pc.purchaseId = \"purchase6558\" ORDER BY l.product",
"SELECT pc.purchaseId, l.product, pd.name, c.lastName, c.firstName FROM purchase pc JOIN customer c USE HASH(probe) ON pc.customerId = c.customerId UNNEST pc.lineItems as l JOIN product pd USE HASH(build) ON l.product = pd.productId WHERE pc.purchaseId = \"purchase6558\" ORDER BY l.product",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p USE HASH(probe) ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p USE HASH(build) ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" AND c.type = \"customer\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p USE HASH(build) ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" AND p.type = \"purchase\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p USE HASH(probe) ON c.customerId = p.customerId WHERE c.lastName = \"Champlin\" AND c.type = \"customer\" AND p.type = \"purchase\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM purchase p JOIN customer c USE HASH(build) ON meta(c).id = p.customerId || \"_\" || p.test_id WHERE p.purchaseId LIKE \"purchase655%\" ORDER BY p.purchaseId",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c JOIN purchase p USE HASH(probe) ON meta(c).id = p.customerId || \"_\" || p.test_id WHERE c.lastName = \"Champlin\" AND p.customerId IS NOT NULL ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c LEFT OUTER JOIN purchase p USE HASH(build) ON c.customerId = p.customerId WHERE c.lastName = \"Wyman\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM purchase p RIGHT OUTER JOIN customer c USE HASH(probe) ON c.customerId = p.customerId WHERE c.lastName = \"Wyman\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM purchase p USE HASH(probe) RIGHT OUTER JOIN customer c ON c.customerId = p.customerId WHERE c.lastName = \"Wyman\" ORDER BY p.purchaseId LIMIT 10",
"SELECT c.firstName, c.lastName, c.customerId, p.purchaseId FROM customer c LEFT OUTER JOIN purchase p USE HASH(probe) ON c.customerId = p.customerId WHERE c.lastName = \"Wyman\" ORDER BY p.purchaseId LIMIT 10",
"SELECT COUNT(1) as mycount FROM product p1 INNER JOIN product p2 ON KEYS (p1.productId) WHERE ANY r IN p1.reviewList SATISFIES r = 'review8565' AND r = 'review8565' END AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\" ",
"SELECT COUNT(1) as mycount FROM product p1 USE INDEX (reviewlistidx2) INNER JOIN product p2 ON KEYS (p1.productId) WHERE ANY r IN p1.reviewList SATISFIES r = 'review8565' AND r = 'review8565' END AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\" ",
"SELECT COUNT(1) as mycount FROM product p1 USE INDEX (reviewlistidx) INNER JOIN product p2 ON KEYS (p1.productId) WHERE ANY r IN p1.reviewList SATISFIES r > 'review8565' END OR ANY r IN p1.reviewList SATISFIES r < 'review1000' END AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\"  ",
"SELECT COUNT(1) as mycount FROM product p1 USE INDEX (reviewlistidx) INNER JOIN product p2 ON KEYS (p1.productId) WHERE ANY r IN p1.reviewList SATISFIES r = 'review8565' AND r = 'review8566' END OR ANY r IN p1.reviewList SATISFIES r = 'review9990' AND r = 'review9991' END AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\" ",
"SELECT COUNT(1) as mycount FROM product p1 USE INDEX (reviewlistidx) INNER JOIN product p2 ON KEYS (p1.productId) WHERE ( ANY r IN p1.reviewList SATISFIES r = 'review8565' AND r = 'review8566' END AND ANY r IN p1.reviewList SATISFIES r = 'review8585' AND r = 'review8586' END) AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\"  ",
"SELECT COUNT(1) as mycount FROM product p1 USE INDEX (reviewlistidx3) INNER JOIN product p2 ON KEYS (p1.productId) WHERE p1.productId IS NOT MISSING AND ANY r IN p1.reviewList SATISFIES r = 'review8565' AND r = 'review8565' END AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\" ",
"SELECT COUNT(1) as mycount FROM product p1 INNER JOIN product p2 ON KEYS (p1.productId) WHERE ANY r IN p1.reviewList SATISFIES r = 'review8565' AND r = 'review8565' END AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\"",
"SELECT COUNT(1) as mycount FROM product p1 USE INDEX (reviewlistidx2all) INNER JOIN product p2 ON KEYS (p1.productId) WHERE ANY r IN p1.reviewList SATISFIES r = 'review8565' AND r = 'review8565' END AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\"",
"SELECT COUNT(1) as mycount FROM product p1 USE INDEX (reviewlistidxall) INNER JOIN product p2 ON KEYS (p1.productId) WHERE ANY r IN p1.reviewList SATISFIES r > 'review8565' END OR ANY r IN p1.reviewList SATISFIES r < 'review1000' END AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\"",
"SELECT COUNT(1) as mycount FROM product p1 USE INDEX (reviewlistidxall) INNER JOIN product p2 ON KEYS (p1.productId) WHERE ANY r IN p1.reviewList SATISFIES r = 'review8565' AND r = 'review8566' END OR ANY r IN p1.reviewList SATISFIES r = 'review9990' AND r = 'review9991' END AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\"",
"SELECT COUNT(1) as mycount FROM product p1 USE INDEX (reviewlistidxall) INNER JOIN product p2 ON KEYS (p1.productId) WHERE ( ANY r IN p1.reviewList SATISFIES r = 'review8565' AND r = 'review8566' END AND ANY r IN p1.reviewList SATISFIES r = 'review8585' AND r = 'review8586' END) AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\"",
"SELECT COUNT(1) as mycount FROM product p1 USE INDEX (reviewlistidx3all) INNER JOIN product p2 ON KEYS (p1.productId) WHERE p1.productId IS NOT MISSING AND ANY r IN p1.reviewList SATISFIES r = 'review8565' AND r = 'review8565' END AND p1.test_id = \"arrayIndex\" and p2.test_id = \"arrayIndex\"",
        "SELECT d1.k0,d1.k1,d2.k3 FROM shellTest d1 JOIN shellTest d2 ON KEYS d1.k1 WHERE d1.k0=1",
        "SELECT meta(b1).id b1id, meta(b2).id b2id FROM shellTest b1 JOIN shellTest b2 ON KEY b2.docid FOR b1 WHERE meta(b1).id > ''",
"SELECT * from default:orders3 INNER JOIN default:contacts ON KEYS orders3.customers ORDER BY orders3.id, contacts.name",
"SELECT * FROM default:orders2 INNER JOIN default:contacts AS cont ON  KEYS orders2.custId ORDER BY orders2.id, cont.name",
"SELECT META(o).id oid FROM default:users_with_orders u USE KEYS \"Adaline_67672807\" INNER JOIN default:users_with_orders o ON KEYS ARRAY s.order_id FOR s IN u.shipped_order_history END ORDER BY oid",
"SELECT META(u).id uid, META(o).id oid FROM default:users_with_orders u USE KEYS \"Aide_48687583\" INNER JOIN default:users_with_orders o ON KEYS ARRAY s.order_id FOR s IN u.shipped_order_history END ORDER BY oid,uid",
"SELECT META(o).id oid FROM default:users_with_orders u USE KEYS \"Adaline_67672807\" UNNEST u.shipped_order_history s INNER JOIN default:users_with_orders o ON KEYS s.order_id ORDER BY oid",
"SELECT o.order_details.order_id AS oid FROM default:users_with_orders u USE KEYS \"Aide_48687583\" INNER JOIN default:users_with_orders o ON KEYS ARRAY s.order_id FOR s IN u.shipped_order_history END ORDER BY oid",
"SELECT  o.order_details.order_id as oid FROM default:users_with_orders u USE KEYS \"Aide_48687583\" UNNEST u.shipped_order_history s INNER JOIN default:users_with_orders o ON KEYS s.order_id ORDER BY oid",
"SELECT META(o).id oid, META(u2).id uid, search.category cat FROM default:users_with_orders u USE KEYS \"Aide_48687583\" UNNEST u.shipped_order_history s INNER JOIN default:users_with_orders o ON KEYS s.order_id INNER JOIN default:users_with_orders u2 ON KEYS META(u).id UNNEST u.search_history search ORDER BY oid, uid",
"SELECT DISTINCT contacts.name AS customer_name, orders3.orderlines FROM default:orders3 INNER JOIN default:contacts ON KEYS orders3.customers ORDER BY customer_name,orders3.orderlines",
"SELECT * from default:orders3 LEFT JOIN default:contacts ON KEYS orders3.customers ORDER BY orders3.id, contacts.name  LIMIT 4",
"SELECT o.order_details.order_id AS oid FROM default:users_with_orders u USE KEYS \"Aide_48687583\" LEFT JOIN default:users_with_orders o ON KEYS ARRAY s.order_id FOR s IN u.shipped_order_history END ORDER BY oid",
"SELECT  o.order_details.order_id as oid FROM default:users_with_orders u USE KEYS \"Aide_48687583\" UNNEST u.shipped_order_history s LEFT JOIN default:users_with_orders o ON KEYS s.order_id ORDER BY oid",
"SELECT META(o).id oid, META(u2).id uid, search.category cat FROM default:users_with_orders u USE KEYS \"Aide_48687583\" UNNEST u.shipped_order_history s LEFT JOIN default:users_with_orders o ON KEYS s.order_id LEFT JOIN default:users_with_orders u2 ON KEYS META(u).id UNNEST u.search_history search ORDER BY oid, cat, uid",
"SELECT DISTINCT contacts.name AS customer_name, orders3.id  FROM default:orders3 LEFT JOIN default:contacts ON KEYS orders3.customers ORDER BY orders3.id,customer_name",
"SELECT META(customer).id oid1, meta(purchase).id oid2 FROM purchase USE KEYS \"purchase0_joins\" INNER JOIN customer ON KEYS purchase.customerId || \"_\" || purchase.test_id where purchase.test_id = \"joins\" order by oid1, oid2",
"SELECT purchase.purchaseId, META(customer).id custID, META(product).id prodID, cardio FROM purchase USE KEYS \"purchase1018_joins\" UNNEST ARRAY (pl.product || \"_\" || \"joins\") FOR pl IN purchase.lineItems END AS pID INNER JOIN product ON KEYS pID INNER JOIN customer ON KEYS (purchase.customerId || \"_\" || \"joins\") UNNEST customer.ccInfo.cardNumber AS cardio ORDER BY productId",
"SELECT pu.customerId, product.unitPrice, product.productId from purchase pu USE KEYS \"purchase1018_joins\" INNER JOIN product ON KEYS ARRAY (pl.product || \"_\" || \"joins\") FOR pl IN pu.lineItems END ORDER BY product.unitPrice DESC",
"SELECT pID, product.unitPrice from purchase pu USE KEYS \"purchase1018_joins\" UNNEST ARRAY (pl.product|| \"_\" || \"joins\") FOR pl IN pu.lineItems END AS pID INNER JOIN product ON KEYS pID ORDER BY pID",
"SELECT META(customer).id oid1, meta(purchase).id oid2 FROM purchase USE KEYS \"purchase0_joins\" LEFT JOIN customer ON KEYS purchase.customerId || \"_\" || purchase.test_id where purchase.test_id = \"joins\" order by oid1, oid2",
"SELECT customer.ccInfo, customer.customerId, purchase.purchaseId, purchase.lineItems from purchase INNER JOIN customer ON KEYS purchase.customerId || \"_\" || purchase.test_id WHERE customer.test_id = \"joins\" ORDER BY purchase.customerId,purchase.purchaseId limit 10",
"SELECT META(customer).id oid1, meta(purchase).id oid2 FROM purchase USE KEYS \"purchase0_joins\" INNER JOIN customer ON KEYS purchase.customerId || \"_\" || purchase.test_id where purchase.test_id = \"joins\" order by oid1, oid2",
"SELECT META(purchase).id purchase_id, META(product).id product_id FROM purchase INNER JOIN product ON KEYS ARRAY s.product || \"_\" || purchase.test_id FOR s IN purchase.lineItems END where purchase.test_id = \"joins\" ORDER BY purchase_id, product_id limit 5",
"SELECT META(purchase).id as purchase_id, meta(product).id as product_id, product.name as name FROM purchase UNNEST purchase.lineItems line INNER JOIN product ON KEYS line.product || \"_\" || purchase.test_id where purchase.test_id = \"joins\" AND product.test_id = \"joins\" ORDER BY purchase_id, product_id, name limit 5 ",
"SELECT purchase.purchaseId, META(customer).id custID, META(product).id prodID, cardio FROM purchase USE KEYS \"purchase1018_joins\" UNNEST ARRAY (pl.product || \"_\" || \"joins\") FOR pl IN purchase.lineItems END AS pID INNER JOIN product ON KEYS pID INNER JOIN customer ON KEYS (purchase.customerId || \"_\" || \"joins\") UNNEST TO_ARRAY(customer.ccInfo.cardNumber) AS cardio ORDER BY prodID",
"SELECT pu.customerId, product.unitPrice, product.productId from purchase pu USE KEYS \"purchase1018_joins\" INNER JOIN product ON KEYS ARRAY (pl.product || \"_\" || \"joins\") FOR pl IN pu.lineItems END ORDER BY product.unitPrice DESC",
"SELECT pID, product.unitPrice from purchase pu USE KEYS \"purchase1018_joins\" UNNEST ARRAY (pl.product|| \"_\" || \"joins\") FOR pl IN pu.lineItems END AS pID INNER JOIN product ON KEYS pID ORDER BY pID",
"SELECT DISTINCT productId, pu.customerId, customer.firstName FROM purchase pu UNNEST ARRAY (pl.product|| \"_\" || \"joins\") FOR pl IN pu.lineItems END AS productId INNER JOIN customer ON KEYS (pu.customerId|| \"_\" || \"joins\") WHERE pu.customerId=\"customer498\" ORDER BY productId limit 8",
"SELECT customer.ccInfo, customer.customerId, purchase.purchaseId, purchase.lineItems from purchase LEFT JOIN customer ON KEYS purchase.customerId || \"_\" || purchase.test_id WHERE customer.test_id = \"joins\" ORDER BY purchase.customerId,purchase.purchaseId limit 10",
"SELECT META(customer).id oid1, meta(purchase).id oid2 FROM purchase USE KEYS \"purchase0_joins\" LEFT JOIN customer ON KEYS purchase.customerId || \"_\" || purchase.test_id where purchase.test_id = \"joins\" order by oid1, oid2",
"SELECT META(purchase).id purchase_id, META(product).id product_id FROM purchase LEFT JOIN product ON KEYS ARRAY s.product || \"_\" || purchase.test_id FOR s IN purchase.lineItems END where purchase.test_id = \"joins\" ORDER BY purchase_id, product_id limit 5",
        "SELECT META(purchase).id as purchase_id, meta(product).id as product_id, product.name as name FROM purchase UNNEST purchase.lineItems line LEFT JOIN product ON KEYS line.product || \"_\" || purchase.test_id where purchase.test_id = \"joins\" AND product.test_id = \"joins\" ORDER BY purchase_id, product_id, name limit 5 ",

'SELECT DISTINCT route.destinationairport FROM `travel-sample` airport JOIN `travel-sample` route ON airport.faa = route.sourceairport AND route.type = "route" WHERE airport.type = "airport" AND airport.city = "San Francisco" AND airport.country = "United States";',
'SELECT hotel.name hotel_name, landmark.name landmark_name, landmark.activity FROM `travel-sample` hotel JOIN `travel-sample` landmark ON hotel.city = landmark.city AND hotel.country = landmark.country AND landmark.type = "landmark" WHERE hotel.type = "hotel" AND hotel.title like "Yosemite%" AND array_length(hotel.public_likes) > 5;',
'SELECT count(*) FROM `travel-sample` airline JOIN `travel-sample` route ON route.airlineid = "airline_" || tostring(airline.id) AND route.type = "route" WHERE airline.type = "airline" AND airline.name = "United Airlines";',
'SELECT DISTINCT airport.airportname FROM `travel-sample` route JOIN `travel-sample` airport ON airport.faa IN [ route.sourceairport, route.destinationairport ] AND airport.type = "airport" WHERE route.type = "route" AND route.airline = "F9" AND route.distance > 3000;',
'SELECT count(*) FROM `travel-sample` airport JOIN `travel-sample` route ON (route.sourceairport = airport.faa OR route.destinationairport = airport.faa) AND route.type = "route" WHERE airport.type = "airport" AND airport.city = "Denver" AND airport.country = "United States";',
'SELECT DISTINCT route.destinationairport FROM `travel-sample` airport JOIN `travel-sample` route USE INDEX(route_airports) ON airport.faa = route.sourceairport AND route.type = "route" WHERE airport.type = "airport" AND airport.city = "San Francisco" AND airport.country = "United States";',
'SELECT airport.airportname, route.airlineid FROM `travel-sample` airport LEFT JOIN `travel-sample` route ON airport.faa = route.sourceairport AND route.type = "route" WHERE airport.type = "airport" AND airport.city = "Denver" AND airport.country = "United States";',
'SELECT airport.airportname, route.airlineid FROM `travel-sample` route RIGHT JOIN `travel-sample` airport ON airport.faa = route.sourceairport AND route.type = "route" WHERE airport.type = "airport" AND airport.city = "Denver" AND airport.country = "United States";',
'SELECT DISTINCT route.destinationairport FROM `travel-sample` airport JOIN `travel-sample` route USE HASH(build) ON airport.faa = route.sourceairport AND route.type = "route" WHERE airport.type = "airport" AND airport.city = "San Jose" AND airport.country = "United States";',
'SELECT DISTINCT route.destinationairport FROM `travel-sample` airport JOIN `travel-sample` route USE HASH(probe) ON airport.faa = route.sourceairport AND route.type = "route" WHERE airport.type = "airport" AND airport.city = "San Jose" AND airport.country = "United States";',
'SELECT DISTINCT route.destinationairport FROM `travel-sample` airport JOIN `travel-sample` route USE HASH(probe) INDEX(route_airports) ON airport.faa = route.sourceairport AND route.type = "route" WHERE airport.type = "airport" AND airport.city = "San Jose" AND airport.country = "United States";',
'SELECT DISTINCT airline.name FROM `travel-sample` airport INNER JOIN `travel-sample` route ON airport.faa = route.sourceairport AND route.type = "route" INNER JOIN `travel-sample` airline ON route.airline = airline.iata AND airline.type = "airline" WHERE airport.type = "airport" AND airport.city = "San Jose" AND airport.country = "United States";',
'SELECT DISTINCT airline.name FROM `travel-sample` airport INNER JOIN `travel-sample` route ON airport.faa = route.sourceairport AND route.type = "route" INNER JOIN `travel-sample` airline USE HASH(build) ON route.airline = airline.iata AND airline.type = "airline" WHERE airport.type = "airport" AND airport.city = "San Jose" AND airport.country = "United States";',
'SELECT DISTINCT airline.name FROM `travel-sample` airport INNER JOIN `travel-sample` route USE HASH(probe) ON airport.faa = route.sourceairport AND route.type = "route" INNER JOIN `travel-sample` airline ON route.airline = airline.iata AND airline.type = "airline" WHERE airport.type = "airport" AND airport.city = "San Jose" AND airport.country = "United States";',
'INSERT INTO default (KEY,VALUE) VALUES("test11_ansijoin", {"c11": 1, "c12": 10, "a11": [ 1, 2, 3, 4 ], "type": "left"}),VALUES("test12_ansijoin", {"c11": 2, "c12": 20, "a11": [ 3, 3, 5, 10 ], "type": "left"}), VALUES("test13_ansijoin", {"c11": 3, "c12": 30, "a11": [ 3, 4, 20, 40 ], "type": "left"}), VALUES("test14_ansijoin", {"c11": 4, "c12": 40, "a11": [ 30, 30, 30 ], "type": "left"});',
'INSERT INTO default (KEY,VALUE) VALUES("test21_ansijoin", {"c21": 1, "c22": 10, "a21": [ 1, 10, 20], "a22": [ 1, 2, 3, 4 ], "type": "right"}), VALUES("test22_ansijoin", {"c21": 2, "c22": 20, "a21": [ 2, 3, 30], "a22": [ 3, 5, 10, 3 ], "type": "right"}), VALUES("test23_ansijoin", {"c21": 2, "c22": 21, "a21": [ 2, 20, 30], "a22": [ 3, 3, 5, 10 ], "type": "right"}), VALUES("test24_ansijoin", {"c21": 3, "c22": 30, "a21": [ 3, 10, 30], "a22": [ 3, 4, 20, 40 ], "type": "right"}), VALUES("test25_ansijoin", {"c21": 3, "c22": 31, "a21": [ 3, 20, 40], "a22": [ 4, 3, 40, 20 ], "type": "right"}), VALUES("test26_ansijoin", {"c21": 3, "c22": 32, "a21": [ 4, 14, 24], "a22": [ 40, 20, 4, 3 ], "type": "right"}), VALUES("test27_ansijoin", {"c21": 5, "c22": 50, "a21": [ 5, 15, 25], "a22": [ 1, 2, 3, 4 ], "type": "right"}), VALUES("test28_ansijoin", {"c21": 6, "c22": 60, "a21": [ 6, 16, 26], "a22": [ 3, 3, 5, 10 ], "type": "right"}), VALUES("test29_ansijoin", {"c21": 7, "c22": 70, "a21": [ 7, 17, 27], "a22": [ 30, 30, 30 ], "type": "right"}), VALUES("test30_ansijoin", {"c21": 8, "c22": 80, "a21": [ 8, 18, 28], "a22": [ 30, 30, 30 ], "type": "right"});',
'SELECT b1.c11, b2.c21, b2.c22 FROM default b1 JOIN default b2 ON b2.c21 = b1.c11 AND ANY v IN b2.a21 SATISFIES v = b1.c12 END AND b2.type = "right" WHERE b1.type = "left";',
'SELECT b1.c11, b2.c21, b2.c22 FROM default b1 UNNEST b1.a11 AS ba1 JOIN default b2 ON ba1 = b2.c21 AND b2.type = "right" WHERE b1.c11 = 2 AND b1.type = "left";',
'SELECT b1.c11, b2.c21, b2.c22 FROM default b1 JOIN default b2 ON b2.c21 IN b1.a11 AND b2.type = "right" WHERE b1.c11 = 2 AND b1.type = "left";',
'SELECT b1.c11, b2.c21, b2.c22 FROM default b1 UNNEST b1.a11 AS ba1 JOIN default b2 ON b2.c21 = b1.c11 AND ANY v IN b2.a21 SATISFIES v = ba1 END AND b2.type = "right" WHERE b1.type = "left";',
'SELECT b1.c11, b2.c21, b2.c22 FROM default b1 JOIN default b2 ON b2.c21 = b1.c11 AND ANY v IN b2.a21 SATISFIES v IN b1.a11 END AND b2.type = "right" WHERE b1.type = "left";',
'SELECT airline.name FROM `travel-sample` route JOIN `travel-sample` airline ON KEYS route.airlineid WHERE route.type = "route" AND route.sourceairport = "SFO" AND route.destinationairport = "JFK";',
'SELECT airline.name FROM `travel-sample` route JOIN `travel-sample` airline ON route.airlineid = meta(airline).id WHERE route.type = "route" AND route.sourceairport = "SFO" AND route.destinationairport = "JFK";',
'SELECT count(*) FROM `travel-sample` airline JOIN `travel-sample` route ON KEY route.airlineid FOR airline WHERE airline.type = "airline" AND route.type = "route" AND airline.name = "United Airlines";',
'SELECT count(*) FROM `travel-sample` airline JOIN `travel-sample` route ON route.airlineid = meta(airline).id WHERE airline.type = "airline" AND route.type = "route" AND airline.name = "United Airlines";',
'SELECT airline.name, ARRAY {"destination": r.destinationairport} FOR r in route END as destinations FROM `travel-sample` airline NEST `travel-sample` route ON airline.iata = route.airline AND route.type = "route" AND route.sourceairport = "SFO" WHERE airline.type = "airline" AND airline.country = "United States";',
*/
    ];


    for (var i=0; i< queries.length; i++) {
        var query = queries[i];
        try {
            console.log("\n\nParsing: \n\n" + query + "\n");
            var result = parser.parse(query);
            console.log("\nresult is: \n\n" + JSON.stringify(result,null,2));
        }
        catch (err) {
            console.log("\n\nParse error for \n\n" + query + "\n\nis: " + err.message);
            console.log(err.stack);
        }
    }
}

function queryFile() {
    var lineReader = require('readline').createInterface({
        input: require('fs').createReadStream('/Users/eben/src/jison/examples/queries.txt')
        //input: require('fs').createReadStream('/Users/eben/src/master/query-ui/query-ui/n1ql_parser/window_queries.n1ql')
    });

    var lineNum = 0;
    lineReader.on('line', function (line) {

        try {
            var result = parser.parse(line);
            //console.log("Parsed line " + ++lineNum + " ok.");
            //console.log("Result: " + JSON.stringify(result));
            //if (result && result[0])
            //  console.log("paths used: \n\n" + JSON.stringify(result[0].pathsUsed,null,2));
        }
        catch (err) {
            console.log("\n\nParse error for \n\n" + line + "\n\nis: " + err.message);
            console.log(err.stack);
        }
    });
}

console.log("Hello world, starting query parsing...");

queryFile();
//queryArray();
