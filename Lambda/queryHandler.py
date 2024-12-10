import sys
import logging
import psycopg2
from psycopg2.extras import RealDictCursor
import os
import json

username = os.environ['USERNAME']
password = os.environ['PASSWORD']
host = os.environ['HOST']
DBname = os.environ['DB_NAME']
port = os.environ['PORT']

logger = logging.getLogger()
logger.setLevel(logging.INFO)

connection = None

def get_connection():
    global connection
    if connection is None or connection.closed:
        try:
            connection = psycopg2.connect(
                dbname=DBname,
                user=username,
                password=password,
                host=host,
                port=port
            )
            logger.info("SUCCESS: Connection to RDS succeeded")
        except Exception as e:
            logger.error("FAILURE: Connection to RDS failed")
            logger.error(e)
            sys.exit(1)
    return connection

def queryHandler(event, context):
    connection = get_connection()
    cursor = connection.cursor(cursor_factory=RealDictCursor)
    sql = event['queryStringParameters']['sql']
    try:
        cursor.execute(sql)
        connection.commit()
        try:
            result = cursor.fetchall()
        except:
            result = []
        logger.info(f"Query Result: {result}")
        return {
            'statusCode': 200,
            "headers": {
                "Content-Type": "application/json"
            },
            'body': json.dumps({
                "result": result
            }, default=str)
        }
    except Exception as e:
        logger.error(e)
        connection.rollback()
        return {
            'statusCode': 400,
            "headers": {
                "Content-Type": "application/json"
            },
            'body': json.dumps({
                "status": "error",
                "message": "Invalid SQL query",
                "details": str(e)
            })
        }
    finally:
        if cursor:
            cursor.close()
