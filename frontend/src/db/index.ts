import { Pool, QueryResultRow } from 'pg'

// Creates a global connection pool
const pool = new Pool({})

export const query = <Result extends QueryResultRow>(
    text: string,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    params: any[] = []
) => {
    return pool.query<Result>(text, params)
}