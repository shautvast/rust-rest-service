drop table if exists blog_entry;
create table blog_entry
(
    created timestamptz,
    title   varchar(100),
    author  varchar(40),
    text    text
);

insert into blog_entry(created, title, author, text)
values (now(), 'Get enterprisey with Rust', 'Sander', 'Lorem Ipsum');
insert into blog_entry(created, title, author, text)
values (now(), 'Get whimsical with data', 'Sander', 'Lorem Ipsum');